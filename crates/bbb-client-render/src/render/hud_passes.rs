//! HUD passes: GUI commands, GUI items, entity previews and the title panorama.

use super::*;

impl Renderer {
    /// GUI entity picture-in-picture targets (vanilla `GuiRenderer.preparePictureInPicture` →
    /// `PictureInPictureRenderer.prepare` + `GuiEntityRenderer`): before the GUI draws, each
    /// sanitized `HudEntityPreview` renders through the entity model pipelines into its own
    /// persistent color+depth PIP target — cleared per preview, GUI-ortho `hud_entity_preview_pip`
    /// camera under `ENTITY_IN_UI` lighting, depth fully isolated from the world depth target.
    /// `collect_hud_draws` then blits each target's color texture into the HUD frame in GUI
    /// submission order (background, then preview blit, then slot items / overlays), matching
    /// vanilla `blitTexture`'s `addBlitToCurrentLayer`.
    pub(super) fn entity_preview_pip_passes(
        &mut self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        glint_elapsed_millis: f64,
        prefix: &mut PresentationPrefix,
    ) {
        let sign_screen = if self
            .hud_sign_editor_screen
            .as_ref()
            .is_some_and(|screen| screen.sign_preview.is_some())
        {
            self.hud_sign_editor_screen.take()
        } else {
            None
        };
        let inventory_screen = if sign_screen.is_none()
            && self
                .hud_inventory_screen
                .as_ref()
                .is_some_and(|screen| !screen.entity_previews.is_empty())
        {
            self.hud_inventory_screen.take()
        } else {
            None
        };
        let previews: &[crate::HudEntityPreview] = if let Some(screen) = sign_screen.as_ref() {
            screen.sign_preview.as_slice()
        } else if let Some(screen) = inventory_screen.as_ref() {
            screen.entity_previews.as_slice()
        } else {
            for target in self.hud_entity_preview_pip_targets.drain(..) {
                target.retire(&self.queue);
            }
            return;
        };
        self.flush_presentation_prefix(frame, stats, prefix);
        let mut retired = std::mem::take(&mut self.hud_entity_preview_pip_targets);
        if retired.len() > previews.len() {
            for extra in retired.drain(previews.len()..) {
                extra.retire(&self.queue);
            }
        }
        let mut reusable = retired.into_iter();
        let mut targets = Vec::with_capacity(previews.len());
        for (preview_index, preview) in previews.iter().enumerate() {
            let mut target = self.ensure_hud_entity_preview_pip_target(
                reusable.next(),
                preview.rect.width,
                preview.rect.height,
            );
            let preview_index =
                u32::try_from(preview_index).expect("sanitized entity preview count fits u32");
            let (entity_draw_calls, item_draw_calls, pipeline_switches) = self
                .encode_hud_entity_preview_pip(
                    frame.encoder,
                    &mut target,
                    preview,
                    preview_index,
                    glint_elapsed_millis,
                );
            stats.entity_model_draw_calls += entity_draw_calls;
            stats.item_model_draw_calls += item_draw_calls;
            stats.pipeline_switches += pipeline_switches;
            targets.push(target);
        }
        self.hud_entity_preview_pip_targets = targets;
        if let Some(screen) = sign_screen {
            self.hud_sign_editor_screen = Some(screen);
        }
        if let Some(screen) = inventory_screen {
            self.hud_inventory_screen = Some(screen);
        }
    }

    pub(super) fn panorama_pass(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        prefix: &mut PresentationPrefix,
    ) {
        let Some(panorama) = &self.pack.hud_panorama else {
            return;
        };
        let rotation_degrees = match (
            self.hud_level_loading_screen.as_ref(),
            self.hud_disconnected_screen.as_ref(),
        ) {
            (Some(loading), _) => match loading.background {
                crate::hud::HudLevelLoadingBackground::Panorama { rotation_degrees } => {
                    rotation_degrees
                }
                _ => return,
            },
            (None, Some(screen)) => screen.panorama_rotation_degrees,
            (None, None) => return,
        };
        self.flush_presentation_prefix(frame, stats, prefix);
        panorama.draw(
            &self.queue,
            frame.encoder,
            frame.view,
            self.surface_size(),
            rotation_degrees,
        );
        stats.panorama_draw_calls += 1;
        stats.pipeline_switches += 1;
    }

    pub(super) fn hud_passes(
        &mut self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        prefix: &mut PresentationPrefix,
    ) -> Result<()> {
        let mut geometry = std::mem::take(&mut self.hud_geometry);
        let mut belt = self.frame_uploads.take_belt();
        self.prepare_hud_geometry(frame.encoder, &mut geometry, &mut belt);
        if self.flush_dynamic_font_atlas() {
            // The dynamic font atlas grew during HUD glyph realization; rebuild HUD geometry
            // once against the resized texture dimensions and bind group.
            geometry.invalidate();
            self.prepare_hud_geometry(frame.encoder, &mut geometry, &mut belt);
            self.flush_dynamic_font_atlas();
        }
        let result = self.draw_hud_geometry(frame, stats, prefix, &geometry);
        self.frame_uploads.restore_belt(belt);
        self.hud_geometry = geometry;
        result
    }

    fn prepare_hud_geometry(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry: &mut crate::hud::HudFrameGeometry,
        belt: &mut crate::frame_buffers::UploadBelt,
    ) {
        let key = self.hud_retained_key();
        if !geometry.can_reuse(key) {
            let mut hud_draws = self.collect_hud_draws_into(std::mem::take(&mut geometry.scratch));
            let frame_varying = crate::hud::hud_frame_varying_used();
            hud_draws.project_vertices(self.gui_surface_size, self.gui_projection_extent());
            let has_hud_vertices = !hud_draws.commands.is_empty() && !hud_draws.vertices.is_empty();
            if has_hud_vertices {
                let next: &[u8] = bytemuck::cast_slice(&hud_draws.vertices);
                let previous: &[u8] = bytemuck::cast_slice(&geometry.uploaded);
                if next != previous || geometry.buffer.buffer().is_none() {
                    geometry
                        .buffer
                        .upload_staged(&self.device, encoder, belt, next);
                    std::mem::swap(&mut geometry.uploaded, &mut hud_draws.vertices);
                }
            }
            geometry.scratch = std::mem::take(&mut hud_draws.vertices);
            geometry.commands.clear();
            if let Some(screen_blur) = hud_draws.screen_blur {
                crate::hud::append_batched_hud_commands(
                    &hud_draws.commands[..screen_blur.first_base_command],
                    &mut geometry.commands,
                );
                let first_base_command = geometry.commands.len();
                crate::hud::append_batched_hud_commands(
                    &hud_draws.commands
                        [screen_blur.first_base_command..hud_draws.post_gui_item_start],
                    &mut geometry.commands,
                );
                let post_gui_item_start = geometry.commands.len();
                let raw_first_overlay =
                    hud_draws.post_gui_item_start + screen_blur.first_overlay_command;
                crate::hud::append_batched_hud_commands(
                    &hud_draws.commands[hud_draws.post_gui_item_start..raw_first_overlay],
                    &mut geometry.commands,
                );
                let first_overlay_command = geometry.commands.len() - post_gui_item_start;
                crate::hud::append_batched_hud_commands(
                    &hud_draws.commands[raw_first_overlay..],
                    &mut geometry.commands,
                );
                geometry.post_gui_item_start = post_gui_item_start;
                geometry.screen_blur = Some(crate::hud::HudScreenBlurDraws {
                    radius: screen_blur.radius,
                    first_base_command,
                    first_overlay_command,
                });
            } else {
                crate::hud::append_batched_hud_commands(
                    &hud_draws.commands[..hud_draws.post_gui_item_start],
                    &mut geometry.commands,
                );
                let post_gui_item_start = geometry.commands.len();
                crate::hud::append_batched_hud_commands(
                    &hud_draws.commands[hud_draws.post_gui_item_start..],
                    &mut geometry.commands,
                );
                geometry.post_gui_item_start = post_gui_item_start;
                geometry.screen_blur = None;
            }
            geometry.has_vertices = has_hud_vertices;
            geometry.frame_varying = frame_varying;
            geometry.retained_key = Some(key);
            geometry.rebuild_count = geometry.rebuild_count.wrapping_add(1);
        }
    }

    fn draw_hud_geometry(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        prefix: &mut PresentationPrefix,
        geometry: &crate::hud::HudFrameGeometry,
    ) -> Result<()> {
        let has_hud_vertices = geometry.has_vertices;
        if let Some(screen_blur) = geometry.screen_blur {
            self.flush_presentation_prefix(frame, stats, prefix);
            if has_hud_vertices {
                let hud_vertex_buffer = geometry.buffer.buffer().expect("uploaded");
                self.draw_hud_command_pass(
                    frame,
                    hud_vertex_buffer,
                    &geometry.commands[..screen_blur.first_base_command],
                    "bbb-client-hud-before-blur-pass",
                    stats,
                );
            }
            self.draw_hud_item_pass(
                frame.encoder,
                frame.view,
                crate::item_models::ItemModelDrawTarget::GuiBeforeBlur,
                "bbb-client-hud-item-before-blur-pass",
                stats,
            );
            if has_hud_vertices {
                let hud_vertex_buffer = geometry.buffer.buffer().expect("uploaded");
                let first_overlay =
                    geometry.post_gui_item_start + screen_blur.first_overlay_command;
                self.draw_hud_command_pass(
                    frame,
                    hud_vertex_buffer,
                    &geometry.commands[geometry.post_gui_item_start..first_overlay],
                    "bbb-client-hud-overlay-before-blur-pass",
                    stats,
                );
            }

            self.screen_blur_passes(frame.encoder, screen_blur.radius, stats)?;

            if has_hud_vertices {
                let hud_vertex_buffer = geometry.buffer.buffer().expect("uploaded");
                self.draw_hud_command_pass(
                    frame,
                    hud_vertex_buffer,
                    &geometry.commands
                        [screen_blur.first_base_command..geometry.post_gui_item_start],
                    "bbb-client-hud-after-blur-pass",
                    stats,
                );
            }
            self.draw_hud_item_pass(
                frame.encoder,
                frame.view,
                crate::item_models::ItemModelDrawTarget::GuiAfterBlur,
                "bbb-client-hud-item-after-blur-pass",
                stats,
            );
            if has_hud_vertices {
                let hud_vertex_buffer = geometry.buffer.buffer().expect("uploaded");
                let first_overlay =
                    geometry.post_gui_item_start + screen_blur.first_overlay_command;
                self.draw_hud_command_pass(
                    frame,
                    hud_vertex_buffer,
                    &geometry.commands[first_overlay..],
                    "bbb-client-hud-overlay-after-blur-pass",
                    stats,
                );
            }
        } else if has_hud_vertices {
            let hud_vertex_buffer = geometry.buffer.buffer().expect("uploaded");
            let base = &geometry.commands[..geometry.post_gui_item_start];
            let overlay = &geometry.commands[geometry.post_gui_item_start..];
            let has_items = self.has_hud_items(crate::item_models::ItemModelDrawTarget::Gui);
            let segments: [&[_]; 2] = if has_items {
                [base, &[]]
            } else {
                [base, overlay]
            };
            self.draw_presentation_hud_pass(
                frame,
                hud_vertex_buffer,
                &segments,
                "bbb-client-hud-pass",
                stats,
                std::mem::take(prefix),
            );
            if has_items {
                self.draw_hud_item_pass(
                    frame.encoder,
                    frame.view,
                    crate::item_models::ItemModelDrawTarget::Gui,
                    "bbb-client-hud-item-pass",
                    stats,
                );
                self.draw_hud_command_pass(
                    frame,
                    hud_vertex_buffer,
                    overlay,
                    "bbb-client-hud-overlay-pass",
                    stats,
                );
            }
        } else {
            self.flush_presentation_prefix(frame, stats, prefix);
            self.draw_hud_item_pass(
                frame.encoder,
                frame.view,
                crate::item_models::ItemModelDrawTarget::Gui,
                "bbb-client-hud-item-pass",
                stats,
            );
        }
        Ok(())
    }

    pub(super) fn draw_hud_command_pass<'a>(
        &'a self,
        frame: &mut FrameColor<'_>,
        vertex_buffer: &'a wgpu::Buffer,
        commands: &[crate::hud::RetainedHudCommand],
        label: &str,
        stats: &mut FrameDrawStats,
    ) {
        self.draw_presentation_hud_pass(
            frame,
            vertex_buffer,
            &[commands],
            label,
            stats,
            PresentationPrefix::default(),
        );
    }

    fn draw_presentation_hud_pass<'a>(
        &'a self,
        frame: &mut FrameColor<'_>,
        vertex_buffer: &'a wgpu::Buffer,
        segments: &[&[crate::hud::RetainedHudCommand]],
        label: &str,
        stats: &mut FrameDrawStats,
        prefix: PresentationPrefix,
    ) {
        if prefix.is_empty() && segments.iter().all(|commands| commands.is_empty()) {
            return;
        }
        let mut pass = frame.begin_pass(label, wgpu::LoadOp::Load, None);
        self.draw_presentation_prefix(&mut pass, &prefix, stats);
        let size = self.physical_surface_size();
        for commands in segments {
            if commands.is_empty() {
                continue;
            }
            // Reset scissor to full surface per segment across retained GUI-item boundaries.
            pass.set_scissor_rect(0, 0, size.width, size.height);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            let (draw_calls, switches) =
                self.draw_retained_hud_commands(&mut pass, vertex_buffer, commands);
            stats.hud_draw_calls += draw_calls;
            stats.pipeline_switches += switches;
        }
    }

    fn has_hud_items(&self, target: crate::item_models::ItemModelDrawTarget) -> bool {
        self.pack
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| draw.target == target)
    }

    pub(super) fn draw_hud_item_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        target: crate::item_models::ItemModelDrawTarget,
        label: &str,
        stats: &mut FrameDrawStats,
    ) {
        if !self.has_hud_items(target) {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.targets.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        self.draw_resident_item_models(
            &mut pass,
            target,
            &[
                crate::item_models::ItemModelDrawPhase::Solid,
                crate::item_models::ItemModelDrawPhase::CutoutNoCull,
                crate::item_models::ItemModelDrawPhase::CutoutZOffset,
                crate::item_models::ItemModelDrawPhase::EndPortal,
                crate::item_models::ItemModelDrawPhase::EndGateway,
                crate::item_models::ItemModelDrawPhase::Pattern,
            ],
            None,
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            target,
            &[crate::item_models::ItemModelDrawPhase::Glint],
            None,
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            target,
            &[
                crate::item_models::ItemModelDrawPhase::Translucent,
                crate::item_models::ItemModelDrawPhase::TranslucentNoCull,
            ],
            None,
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            target,
            &[crate::item_models::ItemModelDrawPhase::GlintTranslucent],
            None,
            stats,
        );
    }
}
