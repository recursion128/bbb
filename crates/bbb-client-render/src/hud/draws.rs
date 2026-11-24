use super::gpu::FULL_UV;
use super::*;
use bbb_client_clock::Elapsed as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HudRetainedKey {
    pub(crate) hud_generation: u64,
    pub(crate) resource_generation: crate::item_models::ItemModelResourceGeneration,
    pub(crate) gui_surface_size: PhysicalSize<u32>,
    pub(crate) gui_projection_extent_bits: [u32; 2],
    pub(crate) gui_scale: u32,
    pub(crate) hud_screen_blur: bool,
    pub(crate) menu_background_blurriness_bits: u32,
    pub(crate) hotbar_block_model_mask: u16,
    pub(crate) pip_target_count: usize,
    pub(crate) font_atlas_ptr: usize,
    pub(crate) item_atlas_ptr: usize,
    pub(crate) gui_sprites_ptr: usize,
    pub(crate) panorama_ptr: usize,
    pub(crate) chat_lines_ptr: usize,
    pub(crate) chat_lines_len: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum RetainedHudCommand {
    Fill {
        start: u32,
        end: u32,
    },
    TextHighlight {
        invert_start: u32,
        additive_start: u32,
        end: u32,
    },
    Scissor {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Sprite {
        bind_group: wgpu::BindGroup,
        start: u32,
        end: u32,
    },
    InvertingSprite {
        bind_group: wgpu::BindGroup,
        start: u32,
        end: u32,
    },
    NauseaOverlay {
        bind_group: wgpu::BindGroup,
        start: u32,
        end: u32,
    },
    TerrainAtlas {
        start: u32,
        end: u32,
    },
    BannerPattern {
        atlas: HudBannerPatternAtlas,
        start: u32,
        end: u32,
    },
    ItemGlint {
        mask_bind_group: wgpu::BindGroup,
        start: u32,
        end: u32,
    },
    MapPreview {
        preview_index: u8,
    },
    EntityPreviewBlit {
        target_index: usize,
        start: u32,
        end: u32,
    },
    LightmapPreview {
        start: u32,
        end: u32,
    },
}

pub(crate) fn append_batched_hud_commands(
    commands: &[HudDrawCommand<'_>],
    out: &mut Vec<RetainedHudCommand>,
) {
    let mut index = 0;
    while index < commands.len() {
        let command = &commands[index];
        index += 1;
        match command {
            HudDrawCommand::Fill { start, end } => {
                let mut end = *end;
                while let Some(HudDrawCommand::Fill {
                    start: next_start,
                    end: next_end,
                }) = commands.get(index)
                {
                    if *next_start != end {
                        break;
                    }
                    end = *next_end;
                    index += 1;
                }
                out.push(RetainedHudCommand::Fill { start: *start, end });
            }
            HudDrawCommand::TextHighlight {
                invert_start,
                additive_start,
                end,
            } => out.push(RetainedHudCommand::TextHighlight {
                invert_start: *invert_start,
                additive_start: *additive_start,
                end: *end,
            }),
            HudDrawCommand::Scissor {
                x,
                y,
                width,
                height,
            } => out.push(RetainedHudCommand::Scissor {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            }),
            HudDrawCommand::Sprite { sprite, start, end } => {
                let mut end = *end;
                while let Some(HudDrawCommand::Sprite {
                    sprite: next,
                    start: next_start,
                    end: next_end,
                }) = commands.get(index)
                {
                    if *next_start != end || !next.same_texture(sprite) {
                        break;
                    }
                    end = *next_end;
                    index += 1;
                }
                out.push(RetainedHudCommand::Sprite {
                    bind_group: sprite.gpu.bind_group.clone(),
                    start: *start,
                    end,
                });
            }
            HudDrawCommand::InvertingSprite { sprite, start, end } => {
                let mut end = *end;
                while let Some(HudDrawCommand::InvertingSprite {
                    sprite: next,
                    start: next_start,
                    end: next_end,
                }) = commands.get(index)
                {
                    if *next_start != end || !next.same_texture(sprite) {
                        break;
                    }
                    end = *next_end;
                    index += 1;
                }
                out.push(RetainedHudCommand::InvertingSprite {
                    bind_group: sprite.gpu.bind_group.clone(),
                    start: *start,
                    end,
                });
            }
            HudDrawCommand::NauseaOverlay { sprite, start, end } => {
                out.push(RetainedHudCommand::NauseaOverlay {
                    bind_group: sprite.gpu.bind_group.clone(),
                    start: *start,
                    end: *end,
                });
            }
            HudDrawCommand::TerrainAtlas { start, end } => {
                out.push(RetainedHudCommand::TerrainAtlas {
                    start: *start,
                    end: *end,
                });
            }
            HudDrawCommand::BannerPattern { atlas, start, end } => {
                out.push(RetainedHudCommand::BannerPattern {
                    atlas: *atlas,
                    start: *start,
                    end: *end,
                });
            }
            HudDrawCommand::ItemGlint { mask, start, end } => {
                out.push(RetainedHudCommand::ItemGlint {
                    mask_bind_group: mask.gpu.bind_group.clone(),
                    start: *start,
                    end: *end,
                });
            }
            HudDrawCommand::MapPreview { preview_index } => {
                out.push(RetainedHudCommand::MapPreview {
                    preview_index: *preview_index,
                });
            }
            HudDrawCommand::EntityPreviewBlit {
                target_index,
                start,
                end,
            } => out.push(RetainedHudCommand::EntityPreviewBlit {
                target_index: *target_index,
                start: *start,
                end: *end,
            }),
            HudDrawCommand::LightmapPreview { start, end } => {
                out.push(RetainedHudCommand::LightmapPreview {
                    start: *start,
                    end: *end,
                });
            }
        }
    }
}

pub(crate) struct HudFrameGeometry {
    pub(crate) buffer: crate::frame_buffers::FrameDataBuffer,
    pub(crate) uploaded: Vec<HudVertex>,
    pub(crate) scratch: Vec<HudVertex>,
    pub(crate) commands: Vec<RetainedHudCommand>,
    pub(crate) post_gui_item_start: usize,
    pub(crate) screen_blur: Option<HudScreenBlurDraws>,
    pub(crate) has_vertices: bool,
    pub(crate) retained_key: Option<HudRetainedKey>,
    pub(crate) frame_varying: bool,
    pub(crate) rebuild_count: u64,
}

impl Default for HudFrameGeometry {
    fn default() -> Self {
        Self {
            buffer: crate::frame_buffers::FrameDataBuffer::vertex("bbb-hud-frame-vertices"),
            uploaded: Vec::new(),
            scratch: Vec::new(),
            commands: Vec::new(),
            post_gui_item_start: 0,
            screen_blur: None,
            has_vertices: false,
            retained_key: None,
            frame_varying: false,
            rebuild_count: 0,
        }
    }
}

impl HudFrameGeometry {
    pub(crate) fn invalidate(&mut self) {
        self.uploaded.clear();
        self.retained_key = None;
    }

    pub(crate) fn can_reuse(&self, key: HudRetainedKey) -> bool {
        !self.frame_varying
            && self.retained_key == Some(key)
            && (!self.has_vertices || self.buffer.buffer().is_some())
    }
}

impl Renderer {
    pub(crate) fn hud_retained_key(&self) -> HudRetainedKey {
        let extent = self.gui_projection_extent();
        let mut hotbar_block_model_mask = 0u16;
        for (slot, model) in self
            .hud_hotbar_block_item_models
            .iter()
            .take(HUD_HOTBAR_SLOTS)
            .enumerate()
        {
            if model.is_some() {
                hotbar_block_model_mask |= 1u16 << slot;
            }
        }
        HudRetainedKey {
            hud_generation: self.hud_generation,
            resource_generation: self.pack.item_model_residency.resource_generation,
            gui_surface_size: self.gui_surface_size,
            gui_projection_extent_bits: [extent[0].to_bits(), extent[1].to_bits()],
            gui_scale: self.gui_scale,
            hud_screen_blur: self.hud_screen_blur,
            menu_background_blurriness_bits: self.menu_background_blurriness.to_bits(),
            hotbar_block_model_mask,
            pip_target_count: self.hud_entity_preview_pip_targets.len(),
            font_atlas_ptr: self
                .pack
                .hud_font_atlas
                .as_ref()
                .map_or(0, |atlas| std::sync::Arc::as_ptr(&atlas.gpu) as usize),
            item_atlas_ptr: self
                .pack
                .item_entity_atlas
                .as_ref()
                .map_or(0, |atlas| std::sync::Arc::as_ptr(&atlas.hud.gpu) as usize),
            gui_sprites_ptr: self.pack.gui_sprites.identity_ptr(),
            panorama_ptr: self.pack.hud_panorama.as_ref().map_or(0, |panorama| {
                std::sync::Arc::as_ptr(&panorama.overlay().gpu) as usize
            }),
            chat_lines_ptr: self.hud_chat_lines.as_ptr() as usize,
            chat_lines_len: self.hud_chat_lines.len(),
        }
    }

    #[cfg(test)]
    pub(crate) fn collect_hud_draws(&self) -> HudDraws<'_> {
        self.collect_hud_draws_into(Vec::new())
    }

    pub(crate) fn collect_hud_draws_into(&self, mut vertices: Vec<HudVertex>) -> HudDraws<'_> {
        reset_hud_frame_varying_used();
        vertices.clear();
        let mut commands = Vec::new();
        let mut post_gui_item_commands = Vec::new();
        let surface_size = self.gui_surface_size();
        let sound_subtitle_placement = self.hud_sound_subtitle_placement();
        // Vanilla `Gui.extractRenderState` extracts nothing
        // at all while `minecraft.screen` is a `LevelLoadingScreen`, so the
        // sleep overlay and subtitles stay hidden with the rest of the HUD.
        if let Some(screen) = self
            .hud_level_loading_screen
            .as_ref()
            .filter(|_| self.hud_command_confirmation.is_none())
        {
            if let (Some(panorama), HudLevelLoadingBackground::Panorama { .. }) =
                (&self.pack.hud_panorama, screen.background)
            {
                disconnected_screen::push_panorama_overlay(
                    &mut vertices,
                    &mut commands,
                    panorama.overlay(),
                    surface_size,
                );
            }
            let screen_blur = (matches!(
                screen.background,
                HudLevelLoadingBackground::Panorama { .. }
            ) && self.hud_screen_blur
                && self.menu_background_blurriness >= 1.0)
                .then_some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: 0,
                });
            level_loading_screen::push_hud_level_loading_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
            self.push_hud_debug_overlay_block(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
            );
            self.push_hud_toast_block(&mut vertices, &mut commands, &mut post_gui_item_commands);
            let post_gui_item_start = commands.len();
            commands.extend(post_gui_item_commands);
            return HudDraws {
                vertices,
                commands,
                post_gui_item_start,
                screen_blur,
            };
        }
        if let Some(screen) = self
            .hud_disconnected_screen
            .as_ref()
            .filter(|_| self.hud_command_confirmation.is_none())
        {
            if let Some(panorama) = &self.pack.hud_panorama {
                disconnected_screen::push_panorama_overlay(
                    &mut vertices,
                    &mut commands,
                    panorama.overlay(),
                    surface_size,
                );
            }
            let screen_blur = (self.pack.hud_panorama.is_some()
                && self.hud_screen_blur
                && self.menu_background_blurriness >= 1.0)
                .then_some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: 0,
                });
            disconnected_screen::push_hud_disconnected_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/menu_background.png"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_book,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
            self.push_hud_debug_overlay_block(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
            );
            self.push_hud_toast_block(&mut vertices, &mut commands, &mut post_gui_item_commands);
            let post_gui_item_start = commands.len();
            commands.extend(post_gui_item_commands);
            return HudDraws {
                vertices,
                commands,
                post_gui_item_start,
                screen_blur,
            };
        }
        if let Some(screen @ HudGameEventScreen::Win(_)) = self
            .hud_game_event_screen
            .as_ref()
            .filter(|_| self.hud_command_confirmation.is_none())
        {
            if sound_subtitle_placement == Some(HudSoundSubtitlePlacement::BeforeScreen) {
                push_hud_sound_subtitles(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    &self.hud_sound_subtitles,
                );
            }
            game_event::push_hud_game_event_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                self.gui_sprite("popup/background"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                self.gui_texture("textures/gui/title/minecraft.png"),
                self.gui_texture("textures/gui/title/minceraft.png"),
                self.gui_texture("textures/gui/title/edition.png"),
                screen,
            );
            self.push_hud_debug_overlay_block(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
            );
            self.push_hud_toast_block(&mut vertices, &mut commands, &mut post_gui_item_commands);
            let post_gui_item_start = commands.len();
            commands.extend(post_gui_item_commands);
            return HudDraws {
                vertices,
                commands,
                post_gui_item_start,
                screen_blur: None,
            };
        }
        let debug_crosshair = (!self.hide_gui)
            .then(|| {
                self.hud_debug_overlay
                    .as_ref()
                    .and_then(|overlay| overlay.debug_crosshair)
            })
            .flatten();

        if !self.hide_gui {
            if let (Some(scope), Some(texture)) = (
                self.hud_spyglass_scope,
                self.gui_texture("textures/misc/spyglass_scope.png"),
            ) {
                push_hud_spyglass_scope(&mut vertices, &mut commands, texture, surface_size, scope);
            }
            if let (Some(confusion), Some(texture)) = (
                self.hud_confusion_overlay,
                self.gui_texture("textures/misc/nausea.png"),
            ) {
                let start = vertices.len() as u32;
                vertices.extend_from_slice(&hud_styled_quad_vertices(
                    surface_size,
                    hud_confusion_overlay_corners(surface_size, confusion.scale()),
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    confusion.tint(),
                ));
                let end = vertices.len() as u32;
                commands.push(HudDrawCommand::NauseaOverlay {
                    sprite: texture,
                    start,
                    end,
                });
            }
            if let Some(portal) = self.hud_portal_overlay {
                let start = vertices.len() as u32;
                vertices.extend_from_slice(&hud_quad_vertices(
                    surface_size,
                    full_screen_hud_rect(surface_size),
                    portal.sprite_uv,
                    [1.0, 1.0, 1.0, portal.alpha()],
                ));
                let end = vertices.len() as u32;
                commands.push(HudDrawCommand::TerrainAtlas { start, end });
            }
            for overlay in &self.hud_camera_overlays {
                let Some(texture) = self.gui_texture(&overlay.texture_id) else {
                    continue;
                };
                push_hud_draw_with_uv_and_tint(
                    &mut vertices,
                    &mut commands,
                    texture,
                    surface_size,
                    full_screen_hud_rect(surface_size),
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    [1.0, 1.0, 1.0, overlay.alpha],
                );
            }
        }

        if let Some(debug_crosshair) = debug_crosshair {
            push_hud_debug_crosshair(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                surface_size,
                debug_crosshair,
            );
        } else if let Some(crosshair) = self
            .gui_sprite("hud/crosshair")
            .filter(|_| !self.hide_gui && self.hud_crosshair_visible)
        {
            push_hud_crosshair_draw(
                &mut vertices,
                &mut commands,
                crosshair,
                surface_size,
                centered_hud_rect(surface_size, crosshair.width, crosshair.height),
                FULL_UV,
            );
            match self.hud_crosshair_attack_indicator {
                HudCrosshairAttackIndicator::None => {}
                HudCrosshairAttackIndicator::Full => {
                    if let Some(full) = self.gui_sprite("hud/crosshair_attack_indicator_full") {
                        let layout = hud_crosshair_attack_indicator_layout(surface_size, 1.0);
                        push_hud_crosshair_draw(
                            &mut vertices,
                            &mut commands,
                            full,
                            surface_size,
                            layout.full,
                            FULL_UV,
                        );
                    }
                }
                HudCrosshairAttackIndicator::Progress { strength_scale } => {
                    if let (Some(background), Some(progress)) = (
                        self.gui_sprite("hud/crosshair_attack_indicator_background"),
                        self.gui_sprite("hud/crosshair_attack_indicator_progress"),
                    ) {
                        let layout =
                            hud_crosshair_attack_indicator_layout(surface_size, strength_scale);
                        push_hud_crosshair_draw(
                            &mut vertices,
                            &mut commands,
                            background,
                            surface_size,
                            layout.background,
                            FULL_UV,
                        );
                        if layout.progress.width > 0 {
                            push_hud_crosshair_draw(
                                &mut vertices,
                                &mut commands,
                                progress,
                                surface_size,
                                layout.progress,
                                layout.progress_uv,
                            );
                        }
                    }
                }
            }
        }

        if !self.hide_gui {
            self.push_hud_hotbar_draws(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
                surface_size,
            );
        }

        match (!self.hide_gui).then_some(&self.hud_contextual_bar) {
            None | Some(HudContextualBar::Empty) => {}
            Some(HudContextualBar::Jump(jump_bar)) => {
                if let Some(background) = self.gui_sprite("hud/jump_bar_background") {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        background,
                        surface_size,
                        experience_bar_hud_rect(surface_size, background.width, background.height),
                    );
                }
                if jump_bar.cooldown {
                    if let Some(cooldown) = self.gui_sprite("hud/jump_bar_cooldown") {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            cooldown,
                            surface_size,
                            experience_bar_hud_rect(surface_size, cooldown.width, cooldown.height),
                        );
                    }
                } else {
                    let progress_width = hud_contextual_bar_progress_width(jump_bar.progress);
                    if progress_width > 0 {
                        if let Some(progress_sprite) = self.gui_sprite("hud/jump_bar_progress") {
                            push_hud_draw_with_uv(
                                &mut vertices,
                                &mut commands,
                                progress_sprite,
                                surface_size,
                                experience_bar_hud_rect(
                                    surface_size,
                                    progress_width,
                                    progress_sprite.height,
                                ),
                                HudUvRect {
                                    min: [0.0, 0.0],
                                    max: [
                                        (progress_width as f32
                                            / progress_sprite.width.max(1) as f32)
                                            .clamp(0.0, 1.0),
                                        1.0,
                                    ],
                                },
                            );
                        }
                    }
                }
            }
            Some(HudContextualBar::Experience { progress }) => {
                if let Some(background) = self.gui_sprite("hud/experience_bar_background") {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        background,
                        surface_size,
                        experience_bar_hud_rect(surface_size, background.width, background.height),
                    );

                    let progress_width = hud_contextual_bar_progress_width(*progress);
                    if progress_width > 0 {
                        if let Some(progress_sprite) =
                            self.gui_sprite("hud/experience_bar_progress")
                        {
                            push_hud_draw_with_uv(
                                &mut vertices,
                                &mut commands,
                                progress_sprite,
                                surface_size,
                                experience_bar_hud_rect(
                                    surface_size,
                                    progress_width,
                                    progress_sprite.height,
                                ),
                                HudUvRect {
                                    min: [0.0, 0.0],
                                    max: [
                                        (progress_width as f32
                                            / progress_sprite.width.max(1) as f32)
                                            .clamp(0.0, 1.0),
                                        1.0,
                                    ],
                                },
                            );
                        }
                    }
                }
            }
            Some(HudContextualBar::Locator(locator)) => {
                // LocatorBarRenderer nine-slices the 12x5 sprite across 182x5.
                if let Some(background) =
                    self.gui_sprite_at_tick("hud/locator_bar_background", locator.client_tick)
                {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        background,
                        surface_size,
                        experience_bar_hud_rect(
                            surface_size,
                            super::layout::HUD_EXPERIENCE_BAR_WIDTH,
                            5,
                        ),
                    );
                }
                for marker in &locator.markers {
                    let sprite_id = self
                        .pack
                        .hud_locator_styles
                        .get(&marker.style_id)
                        .and_then(|style| style.sprite_id_for_distance(marker.distance));
                    let marker_sprite = sprite_id
                        .and_then(|sprite_id| {
                            self.gui_sprite_at_tick(sprite_id, locator.client_tick)
                        })
                        .or_else(|| self.gui_sprite("missingno"));
                    if let Some(marker_sprite) = marker_sprite {
                        push_hud_draw_with_uv_and_tint(
                            &mut vertices,
                            &mut commands,
                            marker_sprite,
                            surface_size,
                            hud_locator_marker_rect(surface_size, marker.angle_degrees),
                            HudUvRect {
                                min: [0.0, 0.0],
                                max: [1.0, 1.0],
                            },
                            hud_argb_to_tint(marker.color_argb),
                        );
                    }

                    let arrow = match marker.pitch {
                        HudLocatorPitch::None => None,
                        HudLocatorPitch::Up => {
                            self.gui_sprite_at_tick("hud/locator_bar_arrow_up", locator.client_tick)
                        }
                        HudLocatorPitch::Down => self
                            .gui_sprite_at_tick("hud/locator_bar_arrow_down", locator.client_tick),
                    };
                    if let Some(arrow) = arrow {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            arrow,
                            surface_size,
                            hud_locator_arrow_rect(
                                surface_size,
                                marker.angle_degrees,
                                matches!(marker.pitch, HudLocatorPitch::Up),
                            ),
                        );
                    }
                }
            }
        }

        // Vanilla `Gui.extractPlayerHealth` extracts the armor row before the
        // hearts; it only draws when `armor > 0`
        // (`Gui.extractArmor`), each of the 10 icons picking
        // full/half/empty per `hud_armor_fill`. The row sits
        // `(numHealthRows - 1) * healthRowHeight + 10` above the heart baseline
        // so multi-row health pushes it up; with no projected
        // health it falls back to a single row (the fixed 10px gap).
        if let Some(armor) = self
            .hud_armor
            .filter(|&armor| armor > 0)
            .filter(|_| !self.hide_gui)
        {
            let (num_health_rows, health_row_height) = self
                .hud_player_health
                .map(hud_health_row_geometry)
                .unwrap_or((1, HUD_SINGLE_HEALTH_ROW_HEIGHT));
            for index in 0..HUD_ARMOR_ICONS_PER_ROW {
                let sprite = match hud_armor_fill(armor, index) {
                    HudIconFill::Empty => self.gui_sprite("hud/armor_empty"),
                    HudIconFill::Half => self.gui_sprite("hud/armor_half"),
                    HudIconFill::Full => self.gui_sprite("hud/armor_full"),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        armor_hud_rect(
                            surface_size,
                            index,
                            num_health_rows,
                            health_row_height,
                            sprite.width,
                            sprite.height,
                        ),
                    );
                }
            }
        }

        // Vanilla `Gui.extractHearts`: ordered blinking
        // container / non-blinking absorption / blinking old-health /
        // non-blinking current-health sprites, with row stacking, regen lift,
        // and low-health shake resolved by `hud_player_heart_instances`.
        if let Some(health) = self.hud_player_health.filter(|_| !self.hide_gui) {
            for instance in hud_player_heart_instances(surface_size, health) {
                if let Some(sprite) = self.hud_heart_sprite(
                    instance.kind,
                    health.hardcore,
                    instance.half,
                    instance.blinking,
                ) {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        instance.rect(sprite.width, sprite.height),
                    );
                }
            }
        }

        // Vanilla `Gui.extractPlayerHealth` resolves the living vehicle's heart
        // count once: a non-zero count replaces the food row
        // and shifts the air row (`getAirBubbleYLine`).
        let vehicle_hearts = (!self.hide_gui)
            .then_some(self.hud_vehicle_health)
            .flatten()
            .map(|vehicle| hud_vehicle_max_hearts(vehicle.max_health))
            .unwrap_or(0);

        if let (0, Some(food)) = (vehicle_hearts, self.hud_food.filter(|_| !self.hide_gui)) {
            let effect = self.hud_food_effect;
            // Vanilla `Gui.extractFood` reseeds/advances `this.random` every
            // frame; bbb reseeds the identical LCG from the render frame counter
            // so the shake flickers deterministically (layout::hud_food_jitter_offsets).
            let jitter = hud_food_jitter_offsets(
                food,
                effect.saturation_empty,
                effect.tick_count,
                crate::text_obfuscation_seed(),
            );
            if let Some(empty) =
                self.hud_food_variant_sprite(HudFoodSprite::Empty, effect.hunger_effect)
            {
                for index in 0..HUD_FOOD_ICONS_PER_ROW {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        empty,
                        surface_size,
                        food_hud_rect(
                            surface_size,
                            index,
                            empty.width,
                            empty.height,
                            jitter[index as usize],
                        ),
                    );
                }

                for index in 0..HUD_FOOD_ICONS_PER_ROW {
                    let sprite =
                        match hud_food_fill(food, index) {
                            HudIconFill::Empty => None,
                            HudIconFill::Half => self
                                .hud_food_variant_sprite(HudFoodSprite::Half, effect.hunger_effect),
                            HudIconFill::Full => self
                                .hud_food_variant_sprite(HudFoodSprite::Full, effect.hunger_effect),
                        };
                    if let Some(sprite) = sprite {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            sprite,
                            surface_size,
                            food_hud_rect(
                                surface_size,
                                index,
                                sprite.width,
                                sprite.height,
                                jitter[index as usize],
                            ),
                        );
                    }
                }
            }
        }

        // Vanilla `Gui.extractAirBubbles` draws after the food row
        // only while under water or below the max supply
        // full bubbles, the one-tick popping frame, and the
        // delayed empty shells (with the all-empty drowning wobble).
        if let Some(air) = self
            .hud_air
            .filter(|_| !self.hide_gui)
            .filter(|air| hud_air_bubbles_visible(air.air, air.max_air, air.eye_in_water))
        {
            let icons = hud_air_bubble_icons(air.air, air.max_air, air.eye_in_water);
            let all_bubbles_empty = icons
                .iter()
                .all(|icon| *icon == Some(HudAirBubbleIcon::Empty));
            let wobble = hud_air_bubble_wobble_offsets(
                all_bubbles_empty,
                air.tick_count,
                crate::text_obfuscation_seed(),
            );
            for index in 0..HUD_AIR_BUBBLES_PER_ROW {
                let (sprite, y_offset) = match icons[index as usize] {
                    Some(HudAirBubbleIcon::Full) => (self.gui_sprite("hud/air"), 0),
                    Some(HudAirBubbleIcon::Popping) => (self.gui_sprite("hud/air_bursting"), 0),
                    Some(HudAirBubbleIcon::Empty) => {
                        (self.gui_sprite("hud/air_empty"), wobble[index as usize])
                    }
                    None => (None, 0),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        air_bubble_hud_rect(
                            surface_size,
                            index,
                            vehicle_hearts,
                            sprite.width,
                            sprite.height,
                            y_offset,
                        ),
                    );
                }
            }
        }

        // Vanilla `Gui.extractVehicleHealth` runs after `extractPlayerHealth`
        // rows of up to 10 hearts stack upward from the
        // food baseline, each heart drawing its container and then the
        // full/half overlay against `ceil(health)`.
        if let Some(vehicle) = self.hud_vehicle_health.filter(|_| vehicle_hearts > 0) {
            let mut remaining_hearts = vehicle_hearts;
            let mut row = 0u32;
            while remaining_hearts > 0 {
                let row_hearts = remaining_hearts.min(HUD_VEHICLE_HEARTS_PER_ROW);
                for index in 0..row_hearts {
                    if let Some(container) = self.gui_sprite("hud/heart/vehicle_container") {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            container,
                            surface_size,
                            vehicle_heart_hud_rect(
                                surface_size,
                                row,
                                index,
                                container.width,
                                container.height,
                            ),
                        );
                    }
                    let overlay = match hud_vehicle_heart_fill(vehicle.health, row, index) {
                        HudIconFill::Empty => None,
                        HudIconFill::Half => self.gui_sprite("hud/heart/vehicle_half"),
                        HudIconFill::Full => self.gui_sprite("hud/heart/vehicle_full"),
                    };
                    if let Some(overlay) = overlay {
                        push_hud_draw(
                            &mut vertices,
                            &mut commands,
                            overlay,
                            surface_size,
                            vehicle_heart_hud_rect(
                                surface_size,
                                row,
                                index,
                                overlay.width,
                                overlay.height,
                            ),
                        );
                    }
                }
                remaining_hearts -= row_hearts;
                row += 1;
            }
        }

        // Vanilla draws the experience level number between the contextual bar
        // background and its render state, i.e. after the
        // status bars and before the boss overlay, gated only on
        // `experienceLevel > 0` (independent of which contextual bar — jump /
        // locator / experience — occupies the slot), so no suppression is
        // needed. It needs the font atlas.
        if let (Some(level), Some(font_atlas)) = (
            self.hud_experience_level.filter(|_| !self.hide_gui),
            &self.pack.hud_font_atlas,
        ) {
            push_hud_experience_level_text(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                font_atlas,
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                level,
            );
        }
        if !self.hide_gui {
            self.push_hud_spectator_action_draws(&mut vertices, &mut commands, surface_size);
        }

        let inventory_effect_panel_visible = self.hud_inventory_shows_mob_effects
            && self.hud_inventory_screen.as_ref().is_some_and(|screen| {
                hud_inventory_mob_effect_panel_width(surface_size, screen).is_some()
            });
        if !self.hide_gui && !inventory_effect_panel_visible {
            push_hud_mob_effect_overlay(
                &mut vertices,
                &mut commands,
                self.gui_sprite("hud/effect_background"),
                self.gui_sprite("hud/effect_background_ambient"),
                &self.pack.gui_sprites,
                surface_size,
                &self.hud_mob_effects,
            );
        }

        // Vanilla `Gui.extractRenderState` submits the boss overlay right
        // after the hotbar/status decorations and before the overlay message
        // / title strata. Per bar: the sprite layers, then
        // the name line — opaque white with the default drop shadow
        // (`graphics.text(..., -1)`, BossHealthOverlay).
        let hud_boss_bars = if self.hide_gui {
            &[][..]
        } else {
            self.hud_boss_bars.as_slice()
        };
        for draw in hud_boss_bar_draws(hud_boss_bars, &self.pack.hud_font_glyphs, surface_size) {
            for layer in &draw.layers {
                if let Some(sprite) = self.hud_boss_bar_sheet_sprite(layer.sheet) {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        boss_bar_hud_rect(surface_size, draw.y, layer.width),
                        hud_boss_bar_fill_uv(layer.width),
                    );
                }
            }
            if let Some(font_atlas) = &self.pack.hud_font_atlas {
                push_hud_screen_text_draw(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    font_atlas,
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    &draw.name,
                );
            }
        }

        // `Gui.extractSleepOverlay` sits outside the hide-GUI branch and
        // advances to a new stratum after boss bars, before overlay
        // message/title/chat/player-list extraction.
        if let Some(sleep_overlay) = self.hud_sleep_overlay {
            push_hud_sleep_overlay(&mut vertices, &mut commands, surface_size, sleep_overlay);
        }

        // Vanilla `Gui.extractRenderState` submits the overlay message and the
        // title/subtitle after the hotbar decorations; open
        // screens render in a later pass, so their draws stay above these.
        if let Some(font_atlas) = self.pack.hud_font_atlas.as_ref().filter(|_| !self.hide_gui) {
            let text_backdrop_opacity = if self.hud_background_for_chat_only {
                0.0
            } else {
                self.hud_text_background_opacity
            };
            if let Some(sidebar) = &self.hud_scoreboard_sidebar {
                scoreboard::push_hud_scoreboard_sidebar(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    font_atlas,
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    sidebar,
                );
            }
            let mut screen_text_draws = Vec::new();
            if let Some(selected_item_name) = &self.hud_selected_item_name {
                screen_text_draws.extend(hud_selected_item_name_draw(
                    selected_item_name,
                    &self.pack.hud_font_book,
                    surface_size,
                    text_backdrop_opacity,
                ));
            }
            if let Some(action_bar) = &self.hud_action_bar_text {
                screen_text_draws.extend(hud_action_bar_text_draw(
                    action_bar,
                    &self.pack.hud_font_book,
                    surface_size,
                    text_backdrop_opacity,
                ));
            }
            if let Some(title) = &self.hud_title_text {
                screen_text_draws.extend(hud_title_text_draws(
                    title,
                    &self.pack.hud_font_book,
                    surface_size,
                    text_backdrop_opacity,
                ));
            }
            for draw in &screen_text_draws {
                push_hud_component_screen_text_draw(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    font_atlas,
                    &self.pack.hud_font_book,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    draw,
                );
            }
            push_hud_chat(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                font_atlas,
                &self.pack.hud_font_book,
                self.gui_sprite("icon/chat_modified"),
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                &self.pack.gui_sprites,
                crate::text_obfuscation_seed(),
                surface_size,
                self.hud_chat_geometry(),
                &self.hud_chat_lines,
                &self.hud_chat_context,
                self.hud_text_background_opacity,
            );
            if let Some(player_list) = &self.hud_player_list {
                player_list::push_hud_player_list(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    font_atlas,
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    self,
                    player_list,
                    &self.hud_player_list_runtime,
                    HudPlayerListHeartSprites {
                        container: self.hud_heart_sprite(
                            HudHeartKind::Container,
                            false,
                            false,
                            false,
                        ),
                        container_blinking: self.hud_heart_sprite(
                            HudHeartKind::Container,
                            false,
                            false,
                            true,
                        ),
                        full: self.hud_heart_sprite(HudHeartKind::Normal, false, false, false),
                        half: self.hud_heart_sprite(HudHeartKind::Normal, false, true, false),
                        full_blinking: self.hud_heart_sprite(
                            HudHeartKind::Normal,
                            false,
                            false,
                            true,
                        ),
                        half_blinking: self.hud_heart_sprite(
                            HudHeartKind::Normal,
                            false,
                            true,
                            true,
                        ),
                        absorbing_full_blinking: self.hud_heart_sprite(
                            HudHeartKind::Absorbing,
                            false,
                            false,
                            true,
                        ),
                        absorbing_half_blinking: self.hud_heart_sprite(
                            HudHeartKind::Absorbing,
                            false,
                            true,
                            true,
                        ),
                    },
                );
            }
        }

        if sound_subtitle_placement == Some(HudSoundSubtitlePlacement::BeforeScreen) {
            push_hud_sound_subtitles(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                &self.hud_sound_subtitles,
            );
        }

        if let Some(screen) = &self.hud_command_confirmation {
            let screen_blur = (self.hud_screen_blur && self.menu_background_blurriness >= 1.0)
                .then_some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            command_confirmation::push_screen(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_book,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
            self.push_hud_debug_overlay_block(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
            );
            self.push_hud_toast_block(&mut vertices, &mut commands, &mut post_gui_item_commands);
            let post_gui_item_start = commands.len();
            commands.extend(post_gui_item_commands);
            return HudDraws {
                vertices,
                commands,
                post_gui_item_start,
                screen_blur,
            };
        }
        let mut screen_blur = None;
        if let Some(screen) = &self.hud_chat_restrictions_screen {
            if self.hud_screen_blur && self.menu_background_blurriness >= 1.0 {
                screen_blur = Some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            }
            push_hud_chat_restrictions_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_command_block_editor_screen {
            push_hud_command_block_editor_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_structure_block_editor_screen {
            push_hud_structure_block_editor_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field"),
                self.gui_sprite("widget/text_field_highlighted"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_jigsaw_block_editor_screen {
            push_hud_jigsaw_block_editor_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field"),
                self.gui_sprite("widget/text_field_highlighted"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                HudJigsawSliderSprites {
                    normal: self.gui_sprite("widget/slider"),
                    highlighted: self.gui_sprite("widget/slider_highlighted"),
                    handle: self.gui_sprite("widget/slider_handle"),
                    handle_highlighted: self.gui_sprite("widget/slider_handle_highlighted"),
                },
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_test_block_editor_screen {
            push_hud_test_block_editor_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field"),
                self.gui_sprite("widget/text_field_highlighted"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_test_instance_block_editor_screen {
            push_hud_test_instance_block_editor_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field"),
                self.gui_sprite("widget/text_field_highlighted"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        // ChatScreen and CommandBlockEditScreen extract their edit box and
        // completion popup after the rest of the screen, so keep suggestions
        // above the immutable screen surface.
        if let Some(suggestions) = &self.hud_command_suggestions {
            push_hud_command_suggestions(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field_highlighted"),
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                self.pack.hud_font_book.non_fishy_glyphs(None),
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                suggestions,
            );
            let input_glyphs = match suggestions.layout {
                HudCommandSuggestionsLayout::Chat => self.pack.hud_font_book.non_fishy_glyphs(None),
                HudCommandSuggestionsLayout::CommandBlock => &self.pack.hud_font_glyphs,
            };
            let preedit = chat_input::preedit_projection(suggestions, surface_size, input_glyphs);
            push_hud_inventory_preedit_overlay(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/preedit"),
                self.pack.hud_font_atlas.as_ref(),
                input_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                preedit.as_ref().map(|projection| &projection.overlay),
            );
        }

        if let Some(screen) = &self.hud_inventory_screen {
            if self.hud_screen_blur && self.menu_background_blurriness >= 1.0 {
                screen_blur = Some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            }
            if self.hud_ingame_ui_present {
                push_hud_pause_background(&mut vertices, &mut commands, surface_size);
            } else if self.hud_screen_blur {
                death_screen::push_menu_background(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    self.gui_texture("textures/gui/inworld_menu_background.png"),
                    surface_size,
                );
            }
            if self.hud_inventory_shows_mob_effects {
                push_hud_inventory_mob_effects(
                    &mut vertices,
                    &mut commands,
                    &self.hud_white_pixel,
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    self.gui_sprite("container/inventory/effect_background"),
                    self.gui_sprite("container/inventory/effect_background_ambient"),
                    &self.pack.gui_sprites,
                    surface_size,
                    screen,
                    &self.hud_mob_effects,
                );
            }
            push_hud_inventory_background_layers_with_dynamic_previews(
                &mut vertices,
                &mut commands,
                self,
                surface_size,
                screen,
                &screen.background_layers,
                &screen.banner_pattern_previews,
                &screen.map_previews,
                &screen.entity_previews,
            );
            if let Some(area) = &screen.scroll_area {
                if push_hud_inventory_scissor(&mut commands, surface_size, screen, area.scissor) {
                    push_hud_inventory_background_layers(
                        &mut vertices,
                        &mut commands,
                        self,
                        surface_size,
                        screen,
                        &area.background_layers,
                    );
                    push_hud_inventory_scissor_reset(&mut commands, surface_size);
                }
            }

            // Vanilla screens submit standard entity previews right after all background blits
            // (`InventoryScreen.renderBg` / `SmithingScreen.renderBg`), and
            // `PictureInPictureRenderer.blitTexture` adds the PIP color texture to the current GUI
            // layer — above the background, below slot highlights / items / overlays. A scissored
            // preview blits only `rect ∩ scissor`, sampling the matching texture sub-rect (vanilla
            // scissors the full-rect blit; for an axis-aligned scissor the two are equivalent).
            // Under wgpu the PIP texture is already GUI-oriented (row 0 = top), so the UVs are
            // identity fractions of the rect — vanilla's `v0=1, v1=0` flip is a GL
            // framebuffer-origin artifact.
            for (target_index, preview) in screen.entity_previews.iter().enumerate() {
                if preview.after_background_layer.is_none() {
                    push_hud_entity_preview_blit(
                        &mut vertices,
                        &mut commands,
                        self,
                        surface_size,
                        screen,
                        target_index,
                        preview,
                    );
                }
            }

            let hovered_slot = screen
                .hovered_slot_id
                .and_then(|slot_id| screen.slots.iter().find(|slot| slot.slot_id == slot_id));

            if let (Some(slot), Some(highlight)) = (
                hovered_slot,
                self.gui_sprite("container/slot_highlight_back"),
            ) {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    highlight,
                    surface_size,
                    inventory_slot_highlight_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        slot.x,
                        slot.y,
                    ),
                );
            }

            let item_atlas = self.hud_item_atlas();
            if let Some(atlas) = item_atlas {
                for slot in &screen.slots {
                    if let Some(icon) = &slot.icon {
                        let item_rect = inventory_slot_item_hud_rect(
                            surface_size,
                            screen.width,
                            screen.height,
                            slot.x,
                            slot.y,
                        );
                        push_hud_item_icon(
                            &mut vertices,
                            &mut commands,
                            atlas,
                            self.pack.hud_font_atlas.as_ref(),
                            &self.pack.hud_digit_glyphs,
                            surface_size,
                            item_rect,
                            icon,
                            slot.block_model.is_none(),
                            slot.block_model.is_none(),
                        );
                        if slot.block_model.is_some() {
                            push_hud_item_icon(
                                &mut vertices,
                                &mut post_gui_item_commands,
                                atlas,
                                self.pack.hud_font_atlas.as_ref(),
                                &self.pack.hud_digit_glyphs,
                                surface_size,
                                item_rect,
                                icon,
                                false,
                                true,
                            );
                        }
                    }
                }
            }

            push_hud_inventory_fill_layers(
                &mut vertices,
                &mut post_gui_item_commands,
                surface_size,
                screen,
                &screen.fill_layers,
                HudInventoryFillStage::BeforeGhostItem,
            );
            if let Some(area) = &screen.scroll_area {
                if push_hud_inventory_scissor(
                    &mut post_gui_item_commands,
                    surface_size,
                    screen,
                    area.scissor,
                ) {
                    push_hud_inventory_fill_layers(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        surface_size,
                        screen,
                        &area.fill_layers,
                        HudInventoryFillStage::BeforeGhostItem,
                    );
                    push_hud_inventory_scissor_reset(&mut post_gui_item_commands, surface_size);
                }
            }
            if let Some(atlas) = item_atlas {
                for item in &screen.ghost_items {
                    let item_rect = inventory_slot_item_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        item.x,
                        item.y,
                    );
                    push_hud_item_icon(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        atlas,
                        self.pack.hud_font_atlas.as_ref(),
                        &self.pack.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        &item.icon,
                        true,
                        false,
                    );
                }
            }
            push_hud_inventory_fill_layers(
                &mut vertices,
                &mut post_gui_item_commands,
                surface_size,
                screen,
                &screen.fill_layers,
                HudInventoryFillStage::AfterGhostItem,
            );
            if let Some(area) = &screen.scroll_area {
                if push_hud_inventory_scissor(
                    &mut post_gui_item_commands,
                    surface_size,
                    screen,
                    area.scissor,
                ) {
                    push_hud_inventory_fill_layers(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        surface_size,
                        screen,
                        &area.fill_layers,
                        HudInventoryFillStage::AfterGhostItem,
                    );
                    push_hud_inventory_scissor_reset(&mut post_gui_item_commands, surface_size);
                }
            }
            if let Some(atlas) = item_atlas {
                for item in screen
                    .ghost_items
                    .iter()
                    .filter(|item| item.draw_decorations)
                {
                    let item_rect = inventory_slot_item_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        item.x,
                        item.y,
                    );
                    push_hud_item_icon(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        atlas,
                        self.pack.hud_font_atlas.as_ref(),
                        &self.pack.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        &item.icon,
                        false,
                        true,
                    );
                }
            }

            if let Some(atlas) = item_atlas {
                for item in &screen.floating_items {
                    let item_rect = inventory_floating_item_hud_rect(surface_size, screen, item);
                    let scissor_rect =
                        inventory_floating_item_scissor_hud_rect(surface_size, screen, item);
                    push_hud_item_icon_clipped(
                        &mut vertices,
                        &mut commands,
                        atlas,
                        self.pack.hud_font_atlas.as_ref(),
                        &self.pack.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        scissor_rect,
                        &item.icon,
                        item.block_model.is_none(),
                        item.draw_decorations && item.block_model.is_none(),
                    );
                    if item.block_model.is_some() {
                        push_hud_item_icon_clipped(
                            &mut vertices,
                            &mut post_gui_item_commands,
                            atlas,
                            self.pack.hud_font_atlas.as_ref(),
                            &self.pack.hud_digit_glyphs,
                            surface_size,
                            item_rect,
                            scissor_rect,
                            &item.icon,
                            false,
                            item.draw_decorations,
                        );
                    }
                }
            }

            push_hud_inventory_fill_layers(
                &mut vertices,
                &mut post_gui_item_commands,
                surface_size,
                screen,
                &screen.fill_layers,
                HudInventoryFillStage::Foreground,
            );
            if let Some(area) = &screen.scroll_area {
                if push_hud_inventory_scissor(
                    &mut post_gui_item_commands,
                    surface_size,
                    screen,
                    area.scissor,
                ) {
                    push_hud_inventory_fill_layers(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        surface_size,
                        screen,
                        &area.fill_layers,
                        HudInventoryFillStage::Foreground,
                    );
                    push_hud_inventory_scissor_reset(&mut post_gui_item_commands, surface_size);
                }
            }

            push_hud_inventory_background_layers(
                &mut vertices,
                &mut post_gui_item_commands,
                self,
                surface_size,
                screen,
                &screen.foreground_layers,
            );

            let scrolling_text_time_seconds = self.started_at.clock_elapsed().as_secs_f64();
            push_hud_inventory_text_labels(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_book,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
                &screen.text_labels,
                None,
                scrolling_text_time_seconds,
            );
            if let Some(area) = &screen.scroll_area {
                if push_hud_inventory_scissor(
                    &mut post_gui_item_commands,
                    surface_size,
                    screen,
                    area.scissor,
                ) {
                    push_hud_inventory_text_labels(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        &self.hud_white_pixel,
                        self.pack.hud_font_atlas.as_ref(),
                        &self.pack.hud_font_book,
                        crate::text_obfuscation_seed(),
                        surface_size,
                        screen,
                        &area.text_labels,
                        Some(area.scissor),
                        scrolling_text_time_seconds,
                    );
                    push_hud_inventory_text_areas(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        &self.hud_white_pixel,
                        self.pack.hud_font_atlas.as_ref(),
                        &self.pack.hud_font_glyphs,
                        &self.pack.hud_obfuscated_glyph_pool,
                        crate::text_obfuscation_seed(),
                        &self.pack.gui_sprites,
                        surface_size,
                        screen,
                        &area.text_areas,
                        area.scissor,
                    );
                    push_hud_inventory_scissor_reset(&mut post_gui_item_commands, surface_size);
                }
                if let Some(scrollbar) = area.scrollbar {
                    push_hud_inventory_scrollbar(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        &self.hud_white_pixel,
                        &self.pack.gui_sprites,
                        surface_size,
                        screen,
                        scrollbar,
                    );
                }
            }

            if let Some(atlas) = item_atlas {
                for item in &screen.foreground_items {
                    let item_rect = inventory_floating_item_hud_rect(surface_size, screen, item);
                    let scissor_rect =
                        inventory_floating_item_scissor_hud_rect(surface_size, screen, item);
                    push_hud_item_icon_clipped(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        atlas,
                        self.pack.hud_font_atlas.as_ref(),
                        &self.pack.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        scissor_rect,
                        &item.icon,
                        item.block_model.is_none(),
                        item.draw_decorations && item.block_model.is_none(),
                    );
                    if item.block_model.is_some() {
                        push_hud_item_icon_clipped(
                            &mut vertices,
                            &mut post_gui_item_commands,
                            atlas,
                            self.pack.hud_font_atlas.as_ref(),
                            &self.pack.hud_digit_glyphs,
                            surface_size,
                            item_rect,
                            scissor_rect,
                            &item.icon,
                            false,
                            item.draw_decorations,
                        );
                    }
                }
            }

            if let (Some(slot), Some(highlight)) = (
                hovered_slot,
                self.gui_sprite("container/slot_highlight_front"),
            ) {
                push_hud_draw(
                    &mut vertices,
                    &mut post_gui_item_commands,
                    highlight,
                    surface_size,
                    inventory_slot_highlight_hud_rect(
                        surface_size,
                        screen.width,
                        screen.height,
                        slot.x,
                        slot.y,
                    ),
                );
            }

            push_hud_inventory_preedit_overlay(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/preedit"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen.preedit_overlay.as_ref(),
            );

            push_hud_inventory_tooltip(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                &self.pack.gui_sprites,
                item_atlas,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_digit_glyphs,
                &self.pack.hud_font_book,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
                self.hud_inventory_bundle_tooltip.as_ref(),
            );
        }

        if let Some(screen) = &self.hud_sign_editor_screen {
            push_hud_sign_editor_screen(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.gui_sprite("widget/preedit"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
                !self.hud_entity_preview_pip_targets.is_empty(),
                match screen.kind {
                    HudSignEditorKind::Hanging { wood } => {
                        self.gui_texture(screen_draws::hanging_sign_texture(wood))
                    }
                    HudSignEditorKind::Standing { .. } => None,
                },
            );
        }

        if let Some(screen) = &self.hud_pause_screen {
            if hud_pause_screen_draws_background(screen) {
                if self.hud_screen_blur && self.menu_background_blurriness >= 1.0 {
                    screen_blur = Some(HudScreenBlurDraws {
                        radius: self.menu_background_blurriness,
                        first_base_command: commands.len(),
                        first_overlay_command: post_gui_item_commands.len(),
                    });
                }
                death_screen::push_menu_background(
                    &mut vertices,
                    &mut post_gui_item_commands,
                    &self.hud_white_pixel,
                    self.gui_texture("textures/gui/inworld_menu_background.png"),
                    surface_size,
                );
            }
            push_hud_pause_screen(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                self.gui_sprite("toast/now_playing"),
                &self.pack.gui_sprites,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
                self.hud_now_playing_toast
                    .as_ref()
                    .filter(|toast| toast.placement == HudNowPlayingToastPlacement::PauseScreen),
            );
        }

        if let Some(screen) = &self.hud_social_interactions_screen {
            if self.hud_screen_blur && self.menu_background_blurriness >= 1.0 {
                screen_blur = Some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            }
            social::push_hud_social_interactions_screen(
                self,
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.gui_sprite("widget/text_field"),
                self.gui_sprite("widget/text_field_highlighted"),
                &self.pack.gui_sprites,
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                social::HudSocialInteractionsSprites {
                    background: self.gui_sprite("social_interactions/background"),
                    search: self.gui_sprite("icon/search"),
                    mute: self.gui_sprite("social_interactions/mute_button"),
                    mute_highlighted: self
                        .gui_sprite("social_interactions/mute_button_highlighted"),
                    unmute: self.gui_sprite("social_interactions/unmute_button"),
                    unmute_highlighted: self
                        .gui_sprite("social_interactions/unmute_button_highlighted"),
                },
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_stats_screen {
            if self.hud_screen_blur && self.menu_background_blurriness >= 1.0 {
                screen_blur = Some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            }
            death_screen::push_menu_background(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                surface_size,
            );
            push_hud_stats_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.hud_item_atlas(),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_digit_glyphs,
                &self.pack.gui_sprites,
                self.gui_texture("textures/gui/menu_background.png"),
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
            let tabs = hud_stats_tab_rects(surface_size);
            let right = tabs[2].0 + tabs[2].2;
            for (path, x, y, width) in [
                (
                    "textures/gui/header_separator.png",
                    0,
                    HUD_STATS_LIST_TOP - 2,
                    tabs[0].0,
                ),
                (
                    "textures/gui/header_separator.png",
                    right,
                    HUD_STATS_LIST_TOP - 2,
                    surface_size.width as i32 - right,
                ),
                (
                    "textures/gui/footer_separator.png",
                    0,
                    surface_size.height as i32 - HUD_STATS_FOOTER_HEIGHT,
                    surface_size.width as i32,
                ),
            ] {
                if let Some(sprite) = self.gui_texture(path) {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        absolute_hud_rect(x as f32, y as f32, width as u32, 2),
                        HudUvRect {
                            min: [0.0, 0.0],
                            max: [width as f32 / 32.0, 1.0],
                        },
                    );
                }
            }
        }

        if let Some(screen) = &self.hud_game_rules_screen {
            game_rules::push_hud_game_rules_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field_highlighted"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_book_screen {
            push_hud_pause_background(&mut vertices, &mut commands, surface_size);
            book::push_hud_book_screen(
                &mut vertices,
                &mut commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/book.png"),
                self.gui_sprite("widget/page_backward"),
                self.gui_sprite("widget/page_backward_highlighted"),
                self.gui_sprite("widget/page_forward"),
                self.gui_sprite("widget/page_forward_highlighted"),
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                &self.pack.gui_sprites,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_book,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_debug_options_screen {
            if let Some(debug_overlay) = &self.hud_debug_overlay {
                push_hud_debug_overlay(
                    &mut vertices,
                    &mut commands,
                    &mut post_gui_item_commands,
                    &self.hud_white_pixel,
                    self.hud_item_atlas(),
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_digit_glyphs,
                    self.gui_texture("textures/gui/container/gamemode_switcher.png"),
                    self.gui_sprite("gamemode_switcher/slot"),
                    self.gui_sprite("gamemode_switcher/selection"),
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    debug_overlay,
                );
                if debug_overlay.show_lightmap_preview {
                    push_hud_debug_lightmap_preview(
                        &mut vertices,
                        &mut post_gui_item_commands,
                        surface_size,
                    );
                }
            }
            push_hud_debug_options_screen(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/text_field"),
                self.gui_sprite("widget/text_field_highlighted"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.gui_sprite("tooltip/background"),
                self.gui_sprite("tooltip/frame"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        } else {
            self.push_hud_debug_overlay_block(
                &mut vertices,
                &mut commands,
                &mut post_gui_item_commands,
            );
        }

        self.push_hud_toast_block(&mut vertices, &mut commands, &mut post_gui_item_commands);

        if sound_subtitle_placement == Some(HudSoundSubtitlePlacement::Deferred) {
            push_hud_sound_subtitles(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                &self.hud_sound_subtitles,
            );
        }

        if let Some(screen) = &self.hud_code_of_conduct_screen {
            if self.hud_screen_blur && self.menu_background_blurriness >= 1.0 {
                screen_blur = Some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            }
            code_of_conduct::push_hud_code_of_conduct_screen(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                self.gui_sprite("widget/text_field"),
                self.gui_sprite("widget/text_field_highlighted"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.gui_sprite("widget/checkbox"),
                self.gui_sprite("widget/checkbox_highlighted"),
                self.gui_sprite("widget/checkbox_selected"),
                self.gui_sprite("widget/checkbox_selected_highlighted"),
                &self.pack.gui_sprites,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(screen) = &self.hud_death_screen {
            if matches!(screen, HudDeathScreen::LinkConfirmation(_))
                && self.hud_screen_blur
                && self.menu_background_blurriness >= 1.0
            {
                screen_blur = Some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            }
            death_screen::push_hud_death_screen(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_book,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                screen,
            );
        }

        if let Some(
            screen @ (HudGameEventScreen::DemoPopup(_)
            | HudGameEventScreen::DemoLinkConfirmation(_)),
        ) = &self.hud_game_event_screen
        {
            if self.hud_screen_blur && self.menu_background_blurriness >= 1.0 {
                screen_blur = Some(HudScreenBlurDraws {
                    radius: self.menu_background_blurriness,
                    first_base_command: commands.len(),
                    first_overlay_command: post_gui_item_commands.len(),
                });
            }
            game_event::push_hud_game_event_screen(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_texture("textures/gui/inworld_menu_background.png"),
                self.gui_sprite("popup/background"),
                HudDebugOptionsButtonSprites::widget(&self.pack.gui_sprites),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                self.gui_texture("textures/gui/title/minecraft.png"),
                self.gui_texture("textures/gui/title/minceraft.png"),
                self.gui_texture("textures/gui/title/edition.png"),
                screen,
            );
        }

        if let Some(preedit) = ime_preedit::active_screen_preedit_projection(self, surface_size) {
            push_hud_inventory_preedit_overlay(
                &mut vertices,
                &mut post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_sprite("widget/preedit"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                Some(&preedit.overlay),
            );
        }

        let post_gui_item_start = commands.len();
        commands.extend(post_gui_item_commands);

        HudDraws {
            vertices,
            commands,
            post_gui_item_start,
            screen_blur,
        }
    }

    pub(super) fn hud_inventory_background_sprite(
        &self,
        texture: HudInventoryBackgroundTexture,
    ) -> Option<&HudSpriteGpu> {
        match texture {
            HudInventoryBackgroundTexture::Sprite(id)
            | HudInventoryBackgroundTexture::FadedSprite { id, .. } => self.gui_sprite(id),
            HudInventoryBackgroundTexture::Texture(path) => self.gui_texture(path),
            HudInventoryBackgroundTexture::MobEffect(effect_id) => {
                self.pack.gui_sprites.mob_effect(effect_id)
            }
            HudInventoryBackgroundTexture::AdvancementLine(_) => None,
        }
    }
}
