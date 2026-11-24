use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HudActivePipeline {
    Fill,
    Sprite,
    InvertingSprite,
    ItemGlint,
    Map,
    NauseaOverlay,
    TextHighlight,
}

impl Renderer {
    #[cfg(test)]
    pub(crate) fn draw_hud_commands<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        hud_vertex_buffer: &'a wgpu::Buffer,
        commands: &'a [crate::hud::HudDrawCommand<'a>],
    ) -> (u64, u64) {
        let mut retained = Vec::with_capacity(commands.len());
        crate::hud::append_batched_hud_commands(commands, &mut retained);
        self.draw_retained_hud_commands(pass, hud_vertex_buffer, &retained)
    }

    pub(crate) fn draw_retained_hud_commands<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        hud_vertex_buffer: &'a wgpu::Buffer,
        commands: &[crate::hud::RetainedHudCommand],
    ) -> (u64, u64) {
        let mut active_pipeline = None;
        let mut scissor_empty = false;
        let mut draw_calls = 0;
        let mut pipeline_switches = 0;
        for command in commands {
            if scissor_empty && !matches!(command, crate::hud::RetainedHudCommand::Scissor { .. }) {
                continue;
            }
            match command {
                crate::hud::RetainedHudCommand::Fill { start, end } => {
                    if active_pipeline != Some(HudActivePipeline::Fill) {
                        pass.set_pipeline(&self.pipelines.hud_gui.fill);
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        pass.set_bind_group(0, &self.pipelines.hud_gui.bind_group, &[]);
                        active_pipeline = Some(HudActivePipeline::Fill);
                        pipeline_switches += 1;
                    }
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::RetainedHudCommand::TextHighlight {
                    invert_start,
                    additive_start,
                    end,
                } => {
                    pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                    pass.set_pipeline(&self.pipelines.hud_gui.invert);
                    pass.set_bind_group(0, &self.pipelines.hud_gui.bind_group, &[]);
                    pass.draw(*invert_start..*additive_start, 0..1);
                    pass.set_pipeline(&self.pipelines.hud_gui.additive);
                    pass.draw(*additive_start..*end, 0..1);
                    active_pipeline = Some(HudActivePipeline::TextHighlight);
                    pipeline_switches += 2;
                    draw_calls += 2;
                }
                crate::hud::RetainedHudCommand::Scissor {
                    x,
                    y,
                    width,
                    height,
                } => {
                    // Clips cross the same integer GUI scale as the projected vertices.
                    let Some((x, y, width, height)) = physical_scissor_from_gui(
                        self.gui_scale,
                        self.surface_size(),
                        *x,
                        *y,
                        *width,
                        *height,
                    ) else {
                        // A clip fully outside the logical GUI has no physical
                        // pixels. wgpu cannot express a zero-area scissor, so
                        // fail closed for this batch: suppress draws until the
                        // next scissor command instead of dropping the clip.
                        scissor_empty = true;
                        continue;
                    };
                    pass.set_scissor_rect(x, y, width, height);
                    scissor_empty = false;
                }
                crate::hud::RetainedHudCommand::Sprite {
                    bind_group,
                    start,
                    end,
                }
                | crate::hud::RetainedHudCommand::InvertingSprite {
                    bind_group,
                    start,
                    end,
                } => {
                    let invert = matches!(
                        command,
                        crate::hud::RetainedHudCommand::InvertingSprite { .. }
                    );
                    let pipeline = if invert {
                        HudActivePipeline::InvertingSprite
                    } else {
                        HudActivePipeline::Sprite
                    };
                    if active_pipeline != Some(pipeline) {
                        pass.set_pipeline(if invert {
                            &self.pipelines.hud_crosshair
                        } else {
                            &self.pipelines.hud
                        });
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        active_pipeline = Some(pipeline);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::RetainedHudCommand::NauseaOverlay {
                    bind_group,
                    start,
                    end,
                } => {
                    if active_pipeline != Some(HudActivePipeline::NauseaOverlay) {
                        pass.set_pipeline(&self.pipelines.hud_nausea_overlay);
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        active_pipeline = Some(HudActivePipeline::NauseaOverlay);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::RetainedHudCommand::TerrainAtlas { start, end } => {
                    if active_pipeline != Some(HudActivePipeline::Sprite) {
                        pass.set_pipeline(&self.pipelines.hud);
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        active_pipeline = Some(HudActivePipeline::Sprite);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, &self.pack.hud_terrain_atlas_bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::RetainedHudCommand::BannerPattern { atlas, start, end } => {
                    let bind_group = match atlas {
                        crate::hud::HudBannerPatternAtlas::StaticEntity => self
                            .pack
                            .entity_model_texture_atlas
                            .as_ref()
                            .map(|atlas| &atlas.hud_bind_group),
                        crate::hud::HudBannerPatternAtlas::DynamicEntity => self
                            .entity_dynamic_player_texture_atlas
                            .as_ref()
                            .map(|atlas| &atlas.hud_bind_group),
                    };
                    let Some(bind_group) = bind_group else {
                        continue;
                    };
                    if active_pipeline != Some(HudActivePipeline::Sprite) {
                        pass.set_pipeline(&self.pipelines.hud);
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        active_pipeline = Some(HudActivePipeline::Sprite);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::RetainedHudCommand::ItemGlint {
                    mask_bind_group,
                    start,
                    end,
                } => {
                    let Some(glint) = &self.pack.item_glint_texture else {
                        continue;
                    };
                    if active_pipeline != Some(HudActivePipeline::ItemGlint) {
                        pass.set_pipeline(&self.pipelines.hud_item_glint);
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        pass.set_bind_group(0, &glint.gui_bind_group, &[]);
                        active_pipeline = Some(HudActivePipeline::ItemGlint);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(1, mask_bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::RetainedHudCommand::MapPreview { preview_index } => {
                    let target = crate::item_models::MapSurfaceTarget::Gui(*preview_index);
                    if !self.has_resident_map_surfaces(target)
                        && !self.has_resident_map_text(target)
                    {
                        continue;
                    }
                    if active_pipeline != Some(HudActivePipeline::Map) {
                        pass.set_pipeline(&self.pack.map_surface_residency.gui_pipeline);
                        pass.set_vertex_buffer(
                            0,
                            self.pack.map_surface_residency.vertex_buffer.slice(..),
                        );
                        pass.set_index_buffer(
                            self.pack.map_surface_residency.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        active_pipeline = Some(HudActivePipeline::Map);
                        pipeline_switches += 1;
                    }
                    draw_calls += self.draw_resident_gui_map_preview(pass, target);
                }
                // Vanilla `PictureInPictureRenderer.blitTexture`: the preview's private PIP color
                // texture draws as a GUI-textured quad on the current layer. The PIP pass rendered
                // the texture earlier this frame (`entity_preview_pip_passes`).
                crate::hud::RetainedHudCommand::EntityPreviewBlit {
                    target_index,
                    start,
                    end,
                } => {
                    let Some(target) = self.hud_entity_preview_pip_targets.get(*target_index)
                    else {
                        continue;
                    };
                    if active_pipeline != Some(HudActivePipeline::Sprite) {
                        pass.set_pipeline(&self.pipelines.hud);
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        active_pipeline = Some(HudActivePipeline::Sprite);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, &target.blit_bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::RetainedHudCommand::LightmapPreview { start, end } => {
                    if active_pipeline != Some(HudActivePipeline::Sprite) {
                        pass.set_pipeline(&self.pipelines.hud);
                        pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                        active_pipeline = Some(HudActivePipeline::Sprite);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, &self.lightmap_hud_bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
            }
        }
        (draw_calls, pipeline_switches)
    }
}
