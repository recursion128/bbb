use std::path::Path;

use anyhow::Result;
use wgpu::util::DeviceExt;

use crate::Renderer;

impl Renderer {
    pub fn render(&mut self, screenshot: Option<&Path>) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(err) => return Err(err.into()),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-native-clear"),
            });

        let mut opaque_draw_calls = 0;
        let mut cutout_draw_calls = 0;
        let mut translucent_draw_calls = 0;
        let mut block_destroy_overlay_draw_calls = 0;
        let mut selection_draw_calls = 0;
        let mut hud_draw_calls = 0;
        let mut pipeline_switches = 0;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-terrain-opaque-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear.into()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if !self.terrain_opaque.is_empty() {
                pass.set_pipeline(&self.terrain_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                for mesh in &self.terrain_opaque {
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                    opaque_draw_calls += 1;
                }
            }
        }

        if !self.terrain_cutout.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-terrain-cutout-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.terrain_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            for mesh in &self.terrain_cutout {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                cutout_draw_calls += 1;
            }
        }

        if !self.terrain_translucent.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-terrain-translucent-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.terrain_translucent_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            for mesh in &self.terrain_translucent {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                translucent_draw_calls += 1;
            }
        }

        if let Some(overlays) = &self.block_destroy_overlays {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-block-destroy-overlay-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.block_destroy_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, overlays.vertex_buffer.slice(..));
            pass.set_index_buffer(overlays.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..overlays.index_count, 0, 0..1);
            block_destroy_overlay_draw_calls += 1;
        }

        if let Some(outline) = &self.selection_outline {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-selection-outline-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.selection_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
            pass.draw(0..outline.vertex_count, 0..1);
            selection_draw_calls += 1;
        }

        {
            let (hud_vertices, hud_commands) = self.collect_hud_draws();
            if !hud_commands.is_empty() {
                let hud_vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-hud-frame-vertices"),
                            contents: bytemuck::cast_slice(&hud_vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-native-hud-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.hud_pipeline);
                pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                pipeline_switches += 1;
                for command in &hud_commands {
                    pass.set_bind_group(0, &command.sprite.bind_group, &[]);
                    pass.draw(command.start..command.end, 0..1);
                }
                hud_draw_calls = hud_commands.len() as u64;
            }
        }

        let readback = if let Some(path) = screenshot {
            Some(self.prepare_screenshot_copy(&mut encoder, &frame.texture, path)?)
        } else {
            None
        };

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        if let Some(readback) = readback {
            self.finish_screenshot(readback)?;
            self.counters.screenshots_written += 1;
        }

        self.counters.frame_index += 1;
        self.counters.opaque_draw_calls = opaque_draw_calls;
        self.counters.cutout_draw_calls = cutout_draw_calls;
        self.counters.translucent_draw_calls = translucent_draw_calls;
        self.counters.block_destroy_overlay_draw_calls = block_destroy_overlay_draw_calls;
        self.counters.selection_draw_calls = selection_draw_calls;
        self.counters.hud_draw_calls = hud_draw_calls;
        self.counters.draw_calls = opaque_draw_calls
            + cutout_draw_calls
            + translucent_draw_calls
            + block_destroy_overlay_draw_calls
            + selection_draw_calls
            + hud_draw_calls;
        self.counters.pipeline_switches = pipeline_switches;
        Ok(())
    }
}
