//! Post-world passes: entity outline chain, transparency combine and blit, camera/screen effects, vignettes and screen blur.

use super::*;

#[derive(Default)]
pub(super) struct PresentationPrefix {
    screen_effect: bool,
    vignette: bool,
}

impl PresentationPrefix {
    pub(super) fn is_empty(&self) -> bool {
        !self.screen_effect && !self.vignette
    }
}

impl Renderer {
    /// Vanilla `LevelRenderer` (`LevelRenderer.java:510-532`) runs the entity outline post chain
    /// in-place on `entityOutlineTarget` during level passes; `GameRenderer`
    /// (`GameRenderer.java:447`, `LevelRenderer.java:232-235`) composites it onto main afterward.
    pub(super) fn entity_outline_post_chain(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) -> Result<()> {
        let Some(post_effect_graphs) = &self.post_effect_graphs else {
            return Ok(());
        };
        if !self.has_entity_outline_content() {
            return Ok(());
        }
        let draw_calls = post_effect_graphs.execute_color_graph(
            &self.device,
            &self.queue,
            encoder,
            VanillaPostEffect::EntityOutline,
            "minecraft:entity_outline",
            &self.targets.entity_outline().view,
            [self.config.width, self.config.height],
            self.post_effect_globals(),
        )?;
        stats.outline_composite_draw_calls += draw_calls;
        stats.pipeline_switches += draw_calls;
        Ok(())
    }

    pub(super) fn entity_outline_composite_pass(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
    ) {
        let mut pass = frame.begin_pass(
            ENTITY_OUTLINE_COMPOSITE_PASS_LABEL,
            wgpu::LoadOp::Load,
            None,
        );
        pass.set_pipeline(&self.pipelines.entity_outline_composite);
        stats.pipeline_switches += 1;
        pass.set_bind_group(0, &self.targets.entity_outline().bind_group, &[]);
        pass.draw(0..3, 0..1);
        stats.outline_composite_draw_calls += 1;
    }

    pub(super) fn transparency_combine_pass(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        frame_plan: &FramePlan,
        direct: bool,
    ) -> Result<()> {
        match frame_plan.transparency_combine {
            FrameTransparencyCombine::None => {}
            FrameTransparencyCombine::VanillaDirect => {
                if direct {
                    self.draw_transparency_combine(
                        frame,
                        "bbb-client-transparency-combine-to-surface-pass",
                        stats,
                    );
                } else {
                    let improved = self.targets.improved();
                    let mut intermediate = FrameColor {
                        encoder: frame.encoder,
                        view: &improved.transparency_final.view,
                    };
                    self.draw_transparency_combine(
                        &mut intermediate,
                        TRANSPARENCY_COMBINE_PASS_LABEL,
                        stats,
                    );
                    let mut pass = frame.begin_pass(
                        TRANSPARENCY_GRAPH_BLIT_PASS_LABEL,
                        wgpu::LoadOp::Load,
                        None,
                    );
                    pass.set_pipeline(&self.transparency_blit_pipeline);
                    pass.set_bind_group(0, &improved.transparency_final.bind_group, &[]);
                    pass.draw(0..3, 0..1);
                    stats.transparency_combine_draw_calls += 1;
                    stats.pipeline_switches += 1;
                }
            }
            FrameTransparencyCombine::GenericGraph => {
                let post_effect_graphs = self
                    .post_effect_graphs
                    .as_ref()
                    .expect("generic transparency plan owns a graph runtime");
                let improved = self.targets.improved();
                let (graph_passes, depth_copy_passes) = post_effect_graphs.execute_transparency(
                    &self.device,
                    &self.queue,
                    frame.encoder,
                    PostEffectTransparencyTargets {
                        main_color: &self.targets.main.view,
                        main_depth: &self.targets.depth.view,
                        translucent_color: &improved.translucent.view,
                        translucent_depth: &improved.translucent.depth.view,
                        item_entity_color: &improved.item_entity.view,
                        item_entity_depth: &improved.item_entity.depth.view,
                        particles_color: &improved.particle.view,
                        particles_depth: &improved.particle.depth.view,
                        clouds_color: &improved.cloud.view,
                        clouds_depth: &improved.cloud.depth.view,
                        weather_color: &improved.weather.view,
                        weather_depth: &improved.weather.depth.view,
                        size: [self.config.width, self.config.height],
                    },
                    self.post_effect_globals(),
                )?;
                stats.transparency_combine_draw_calls += graph_passes;
                stats.post_effect_depth_copy_draw_calls += depth_copy_passes;
                stats.pipeline_switches += graph_passes + depth_copy_passes;
            }
        }
        Ok(())
    }

    // The matched Vanilla graph ends in a same-size nearest blit with unit color modulation.
    fn draw_transparency_combine(
        &self,
        frame: &mut FrameColor<'_>,
        label: &str,
        stats: &mut FrameDrawStats,
    ) {
        let mut pass = frame.begin_pass(label, wgpu::LoadOp::Clear(wgpu::Color::BLACK), None);
        self.record_transparency_combine(&mut pass, &self.transparency_combine_pipeline, stats);
    }

    pub(super) fn record_transparency_combine(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        stats: &mut FrameDrawStats,
    ) {
        stats.transparency_combine_draw_calls += 1;
        stats.pipeline_switches += 1;
        let omitted = stats.omitted_transparency_targets.mask();
        pass.set_pipeline(pipeline);
        pass.set_bind_group(
            0,
            &self.targets.improved().transparency_combine.bind_group,
            &[],
        );
        // One fullscreen triangle carries the per-draw layer mask in its instance index.
        pass.draw(0..3, omitted..omitted + 1);
    }

    /// Vanilla screen effects precede the camera post effect, vignette and ordinary HUD.
    pub(super) fn prepare_presentation_prefix(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> PresentationPrefix {
        if self.screen_effect_quads.is_empty() {
            return PresentationPrefix::default();
        }
        let surface_size = self.physical_surface_size();
        let aspect = surface_size.width as f32 / surface_size.height as f32;
        let projection = (self.camera_hud_fov_degrees, aspect);
        if self.screen_effect_projection != Some(projection) {
            let (vertices, batches) = crate::screen_effects::screen_effect_draw_data(
                &self.screen_effect_quads,
                self.camera_hud_fov_degrees,
                aspect,
            );
            self.screen_effect_vertices.upload_staged(
                &self.device,
                encoder,
                self.frame_uploads.belt(),
                bytemuck::cast_slice(&vertices),
            );
            self.screen_effect_batches = batches;
            self.screen_effect_projection = Some(projection);
        }
        PresentationPrefix {
            screen_effect: true,
            vignette: false,
        }
    }

    pub(super) fn flush_presentation_prefix(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        prefix: &mut PresentationPrefix,
    ) {
        if prefix.is_empty() {
            return;
        }
        let prefix = std::mem::take(prefix);
        let mut pass = frame.begin_pass(
            "bbb-client-presentation-color-pass",
            wgpu::LoadOp::Load,
            None,
        );
        self.draw_presentation_prefix(&mut pass, &prefix, stats);
    }

    pub(super) fn draw_presentation_prefix<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        prefix: &'a PresentationPrefix,
        stats: &mut FrameDrawStats,
    ) {
        if prefix.screen_effect {
            pass.set_pipeline(&self.pipelines.screen_effect);
            stats.pipeline_switches += 1;
            let vertices = self
                .screen_effect_vertices
                .buffer()
                .expect("prepared screen effect");
            pass.set_vertex_buffer(0, vertices.slice(..));
            for batch in &self.screen_effect_batches {
                let bind_group = match batch.texture {
                    crate::screen_effects::ScreenEffectTexture::TerrainAtlas => {
                        &self.pack.hud_terrain_atlas_bind_group
                    }
                    crate::screen_effects::ScreenEffectTexture::Underwater => {
                        let Some(underwater) = self.screen_effect_underwater.as_ref() else {
                            continue;
                        };
                        &underwater.bind_group
                    }
                };
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(
                    batch.first_vertex..batch.first_vertex + batch.vertex_count,
                    0..1,
                );
                stats.hud_draw_calls += 1;
            }
        }
        if prefix.vignette {
            let texture = self
                .pack
                .hud_vignette_texture
                .as_ref()
                .expect("prepared vignette");
            pass.set_pipeline(&self.pipelines.vignette);
            pass.set_bind_group(0, &texture.bind_group, &[]);
            pass.draw(0..3, 0..1);
            stats.pipeline_switches += 1;
            stats.hud_draw_calls += 1;
        }
    }

    pub(super) fn camera_post_effect_pass(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        prefix: &mut PresentationPrefix,
    ) -> Result<()> {
        let (Some(post_effect_graphs), Some(effect)) = (
            &self.post_effect_graphs,
            self.camera_post_effect_active
                .then(|| VanillaPostEffect::camera_from_id(self.current_post_effect_id.as_deref()))
                .flatten(),
        ) else {
            return Ok(());
        };
        self.flush_presentation_prefix(frame, stats, prefix);
        let draw_calls = post_effect_graphs.execute_color_graph(
            &self.device,
            &self.queue,
            frame.encoder,
            effect,
            "minecraft:main",
            frame.view,
            [self.config.width, self.config.height],
            self.post_effect_globals(),
        )?;
        stats.post_effect_draw_calls += draw_calls;
        stats.pipeline_switches += draw_calls;
        Ok(())
    }

    pub(super) fn prepare_vignette(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        prefix: &mut PresentationPrefix,
    ) {
        let (Some(vignette), Some(texture)) =
            (self.hud_vignette, self.pack.hud_vignette_texture.as_mut())
        else {
            return;
        };
        let [r, g, b] = vignette.color;
        texture.set_color(encoder, self.frame_uploads.belt(), [r, g, b, 1.0]);
        prefix.vignette = true;
    }

    pub(super) fn win_screen_portal_pass(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        prefix: &mut PresentationPrefix,
    ) {
        let win_screen = matches!(
            self.hud_game_event_screen.as_ref(),
            Some(crate::hud::HudGameEventScreen::Win(_))
        );
        let end_portal_level_load = self
            .hud_level_loading_screen
            .as_ref()
            .is_some_and(|screen| {
                screen.background == crate::hud::HudLevelLoadingBackground::EndPortal
            });
        if !win_screen && !end_portal_level_load {
            return;
        }
        let (Some(mesh), Some(atlas)) = (
            self.pack.hud_win_portal_mesh.as_ref(),
            self.pack.entity_model_texture_atlas.as_ref(),
        ) else {
            return;
        };
        self.flush_presentation_prefix(frame, stats, prefix);
        let mut pass = frame.begin_pass(WIN_SCREEN_PORTAL_PASS_LABEL, wgpu::LoadOp::Load, None);
        pass.set_pipeline(&self.pipelines.hud_win_portal);
        pass.set_bind_group(0, &atlas.bind_group, &[]);
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        stats.pipeline_switches += 1;
        stats.hud_draw_calls += 1;
    }

    pub(super) fn win_screen_vignette_pass(
        &mut self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
    ) {
        if !matches!(
            self.hud_game_event_screen.as_ref(),
            Some(crate::hud::HudGameEventScreen::Win(_))
        ) {
            return;
        }
        let Some(texture) = self.pack.hud_win_credits_vignette.as_mut() else {
            return;
        };
        texture.set_color(frame.encoder, self.frame_uploads.belt(), [1.0; 4]);
        let bind_group = &texture.bind_group;
        let mut pass = frame.begin_pass(WIN_SCREEN_VIGNETTE_PASS_LABEL, wgpu::LoadOp::Load, None);
        pass.set_pipeline(&self.pipelines.vignette);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
        stats.pipeline_switches += 1;
        stats.hud_draw_calls += 1;
    }

    /// Vanilla `GameRenderer.processBlurEffect`: the blur post chain over the main target.
    pub(super) fn screen_blur_passes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        radius: f32,
        stats: &mut FrameDrawStats,
    ) -> Result<()> {
        let Some(post_effect_graphs) = &self.post_effect_graphs else {
            return Ok(());
        };
        let draw_calls = post_effect_graphs.execute_color_graph(
            &self.device,
            &self.queue,
            encoder,
            VanillaPostEffect::Blur,
            "minecraft:main",
            &self.targets.main.view,
            [self.config.width, self.config.height],
            PostEffectGlobals {
                menu_blur_radius: radius.round() as i32,
                ..self.post_effect_globals()
            },
        )?;
        stats.screen_blur_draw_calls += draw_calls;
        stats.pipeline_switches += draw_calls;
        Ok(())
    }

    pub(crate) fn post_effect_globals(&self) -> PostEffectGlobals {
        // Vanilla `GlobalSettingsUniform.update` (`GlobalSettingsUniform.java:28-33`) splits
        // `cameraRenderState.pos` (eye position, `GameRenderer.java:431-441`) into
        // `(floor(pos), floor(pos) - pos)` so `CameraOffset` lies in `(-1, 0]`.
        let split_coordinate = |value: f32| {
            if value.is_finite() {
                let block = value.floor() as i32;
                (block, block as f32 - value)
            } else {
                (0, 0.0)
            }
        };
        let position = self
            .camera_pose
            .map(|pose| {
                [
                    pose.position[0],
                    pose.position[1] + pose.eye_height,
                    pose.position[2],
                ]
            })
            .unwrap_or([0.0; 3]);
        let (block_x, offset_x) = split_coordinate(position[0]);
        let (block_y, offset_y) = split_coordinate(position[1]);
        let (block_z, offset_z) = split_coordinate(position[2]);
        let game_time = if self.shader_game_time_ticks.is_finite() {
            (self.shader_game_time_ticks.max(0.0) % 24_000.0) as f32 / 24_000.0
        } else {
            0.0
        };
        PostEffectGlobals {
            camera_block_pos: [block_x, block_y, block_z],
            camera_offset: [offset_x, offset_y, offset_z],
            glint_alpha: self.glint_alpha,
            game_time,
            menu_blur_radius: self.menu_background_blurriness.round() as i32,
            use_rgss: self.terrain_texture_filtering
                == crate::renderer::TerrainTextureFiltering::Rgss,
        }
    }

    pub(super) fn transparency_blit_pass(
        &self,
        frame: &FrameTarget,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        if self.config.usage.contains(wgpu::TextureUsages::COPY_DST) {
            // The main target shares the surface's format and extent; no sampling is needed.
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.main._texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: frame.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
            );
            return;
        }
        let surface_view = frame
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = &self.targets.main_presentation;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(TRANSPARENCY_BLIT_PASS_LABEL),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
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
        pass.set_pipeline(&self.transparency_blit_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
        stats.pipeline_switches += 1;
        stats.transparency_blit_draw_calls += 1;
    }
}
