//! World pass boundaries: regular main attachments, improved targets, and outline dependencies.

use super::translucent_targets::PreparedWorldTargets;
use super::world_features::PreparedWorldFeatures;
use super::*;

impl Renderer {
    pub(super) fn render_world_passes(
        &mut self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        frame_plan: &FramePlan,
        particles: &ParticleBatches,
        gpu_frame: Option<crate::gpu_timing::GpuFrameToken>,
    ) -> Result<()> {
        let targets = self.prepare_world_targets(frame.encoder, particles);
        let features = self.prepare_world_features(frame.encoder);
        if frame_plan.improved_transparency {
            self.improved_world_passes(frame, stats, frame_plan, particles, &features, &targets)
        } else {
            self.regular_world_passes(frame, stats, frame_plan, particles, &features, &targets)
        }?;
        if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
            timing.mark_stage(frame.encoder, token, GpuFrameStage::WorldEnd);
        }
        Ok(())
    }

    fn begin_main_world_pass<'a>(
        &self,
        frame: &'a mut FrameColor<'_>,
        clear: bool,
        label: &'static str,
    ) -> wgpu::RenderPass<'a> {
        let (color, depth) = if clear {
            (
                wgpu::LoadOp::Clear(self.clear.into()),
                wgpu::LoadOp::Clear(1.0),
            )
        } else {
            (wgpu::LoadOp::Load, wgpu::LoadOp::Load)
        };
        frame.begin_pass(label, color, Some((&self.targets.depth.view, depth)))
    }

    fn regular_world_passes<'a>(
        &'a self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        frame_plan: &FramePlan,
        particles: &'a ParticleBatches,
        features: &'a PreparedWorldFeatures,
        targets: &PreparedWorldTargets,
    ) -> Result<()> {
        let outline = self.has_entity_outline_content();
        let mut pass = self.begin_main_world_pass(frame, true, "bbb-client-regular-world-pass");
        self.draw_main_world(&mut pass, stats, features);
        self.draw_opaque_particles(&mut pass, stats, particles, targets);
        self.draw_entity_shadows(&mut pass, stats);
        if self.has_early_world_features(features) {
            self.draw_early_world_features(&mut pass, stats, features);
        }
        self.draw_item_entity_target(&mut pass, stats, targets);
        self.draw_block_destroy_overlays(&mut pass, stats);
        self.draw_ordered_world_text(&mut pass, stats, features);
        if outline {
            drop(pass);
            self.entity_outline_target_pass(frame.encoder, stats);
            pass = self.begin_main_world_pass(
                frame,
                false,
                "bbb-client-regular-world-after-outline-target-pass",
            );
        }
        self.draw_selection_before_translucent(&mut pass, stats);
        self.draw_translucent_terrain(&mut pass, stats);
        if self.has_item_entity_lines(targets) {
            self.draw_item_entity_lines(&mut pass, stats, targets);
        }
        if frame_plan.particle_depth_work {
            self.draw_particle_target(&mut pass, stats, particles, targets);
        }
        if outline && self.post_effect_graphs.is_some() {
            drop(pass);
            self.entity_outline_post_chain(frame.encoder, stats)?;
            pass = self.begin_main_world_pass(
                frame,
                false,
                "bbb-client-regular-world-after-outline-chain-pass",
            );
        }
        if self.has_cloud_draws() {
            self.draw_clouds(&mut pass, stats);
        }
        self.draw_weather(&mut pass, stats, targets, false);
        Ok(())
    }

    fn improved_world_passes<'a>(
        &'a mut self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        frame_plan: &FramePlan,
        particles: &'a ParticleBatches,
        features: &'a PreparedWorldFeatures,
        targets: &PreparedWorldTargets,
    ) -> Result<()> {
        if frame_plan.transparency_combine == FrameTransparencyCombine::VanillaDirect {
            stats.omitted_transparency_targets = self.omitted_transparency_targets(targets);
        }
        let omitted = stats.omitted_transparency_targets;
        {
            let mut pass =
                self.begin_main_world_pass(frame, true, "bbb-client-terrain-opaque-group-pass");
            self.draw_main_world(&mut pass, stats, features);
            self.draw_opaque_particles(&mut pass, stats, particles, targets);
            // Shadows blend onto main without changing the opaque depth copied below.
            self.draw_entity_shadows(&mut pass, stats);
        }
        self.copy_main_depth_to_feature_targets(frame.encoder, omitted);
        if self.has_early_world_features(features) {
            let mut pass =
                self.begin_main_world_pass(frame, false, ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL);
            self.draw_early_world_features(&mut pass, stats, features);
        }
        if !omitted.item_entity {
            let improved = self.targets.improved();
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some(ITEM_ENTITY_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &improved.item_entity.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: (frame_plan.item_entity_depth_work).then_some(
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &improved.item_entity.depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    },
                ),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };
            let mut pass = frame.encoder.begin_render_pass(&descriptor);
            self.draw_item_entity_target(&mut pass, stats, targets);
        }
        if self.has_late_world_features(features) {
            let mut pass =
                self.begin_main_world_pass(frame, false, "bbb-client-ordered-world-features-pass");
            self.draw_block_destroy_overlays(&mut pass, stats);
            self.draw_ordered_world_text(&mut pass, stats, features);
        }
        self.entity_outline_target_pass(frame.encoder, stats);
        if self
            .block_selection
            .has_phase(BlockSelectionPhase::BeforeTranslucent)
        {
            let improved = self.targets.improved();
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some(BLOCK_SELECTION_BEFORE_TRANSLUCENT_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &improved.item_entity.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &improved.item_entity.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };
            let mut pass = frame.encoder.begin_render_pass(&descriptor);
            self.draw_selection_before_translucent(&mut pass, stats);
        }
        let tail = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-client-translucent-and-presentation"),
            });
        self.frame_uploads.finish_belt();
        self.terrain_finish
            .as_mut()
            .unwrap()
            .start(std::mem::replace(frame.encoder, tail));
        {
            let improved = self.targets.improved();
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some(TRANSLUCENT_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &improved.translucent.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &improved.translucent.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };
            let mut pass = frame.encoder.begin_render_pass(&descriptor);
            self.draw_translucent_terrain(&mut pass, stats);
        }
        if self.has_item_entity_lines(targets) {
            let improved = self.targets.improved();
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some(ITEM_ENTITY_LINE_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &improved.item_entity.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &improved.item_entity.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };
            let mut pass = frame.encoder.begin_render_pass(&descriptor);
            self.draw_item_entity_lines(&mut pass, stats, targets);
        }
        if !omitted.particles {
            let improved = self.targets.improved();
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some(PARTICLE_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &improved.particle.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: (frame_plan.particle_depth_work).then_some(
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &improved.particle.depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    },
                ),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };
            let mut pass = frame.encoder.begin_render_pass(&descriptor);
            self.draw_particle_target(&mut pass, stats, particles, targets);
        }
        self.entity_outline_post_chain(frame.encoder, stats)?;
        if !omitted.clouds {
            let improved = self.targets.improved();
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some(CLOUDS_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &improved.cloud.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &improved.cloud.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };
            let mut pass = frame.encoder.begin_render_pass(&descriptor);
            if self.has_cloud_draws() {
                self.draw_clouds(&mut pass, stats);
            }
        }
        if !omitted.weather {
            let improved = self.targets.improved();
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some(WEATHER_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &improved.weather.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &improved.weather.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };
            let mut pass = frame.encoder.begin_render_pass(&descriptor);
            self.draw_weather(&mut pass, stats, targets, true);
        }
        Ok(())
    }

    fn copy_main_depth_to_feature_targets(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        omitted: super::translucent_targets::OmittedTransparencyTargets,
    ) {
        // LevelRenderer copies opaque depth before translucent features.
        // A custom graph can read a target's depth even when the layer has no draws.
        let source = &self.targets.depth._texture;
        let improved = self.targets.improved();
        for target in [
            Some(&improved.translucent.depth._texture),
            (!omitted.item_entity).then_some(&improved.item_entity.depth._texture),
            (!omitted.particles).then_some(&improved.particle.depth._texture),
        ]
        .into_iter()
        .flatten()
        {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::DepthOnly,
                    ..source.as_image_copy()
                },
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::DepthOnly,
                    ..target.as_image_copy()
                },
                source.size(),
            );
        }
    }
}
