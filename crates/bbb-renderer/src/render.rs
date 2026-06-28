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
        let mut entity_model_draw_calls = 0;
        let mut particle_draw_calls = 0;
        let mut item_entity_draw_calls = 0;
        let mut item_model_draw_calls = 0;
        let mut selection_draw_calls = 0;
        let mut entity_scene_draw_calls = 0;
        let mut entity_target_draw_calls = 0;
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
            if let Some(mesh) = &self.entity_model_mesh {
                pass.set_pipeline(&self.entity_model_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_textured_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_skin_cutout_mesh,
                &self.entity_dynamic_player_skin_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_texture_cutout_mesh,
                &self.entity_dynamic_player_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_translucent_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_translucent_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_skin_translucent_mesh,
                &self.entity_dynamic_player_skin_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_translucent_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_texture_translucent_mesh,
                &self.entity_dynamic_player_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_translucent_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_eyes_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_eyes_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            // The scrolling overlays draw last, over the already-shaded entity bodies: the translucent
            // `breezeWind` (wind charge) then the additive `energySwirl` (charged-creeper / wither glow).
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_scroll_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_scroll_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_scroll_additive_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_scroll_additive_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
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

        // Block-item models sample the blocks atlas (the terrain bind group); flat / generated item
        // models sample the item atlas (the dropped-item billboard bind group). Each is a separate draw
        // because they bind a different texture, but both reuse the one item-model pipeline.
        let (block_item_vertices, block_item_indices) = self.collect_block_item_model_geometry();
        if !block_item_indices.is_empty() {
            self.draw_item_model_geometry(
                &mut encoder,
                &view,
                &block_item_vertices,
                &block_item_indices,
                &self.terrain_bind_group,
            );
            pipeline_switches += 1;
            item_model_draw_calls += 1;
        }
        let (map_vertices, map_indices) = self.collect_item_frame_map_geometry();
        if !map_indices.is_empty() {
            if let Some(atlas) = &self.item_frame_map_atlas {
                self.draw_item_model_geometry(
                    &mut encoder,
                    &view,
                    &map_vertices,
                    &map_indices,
                    &atlas.bind_group,
                );
                pipeline_switches += 1;
                item_model_draw_calls += 1;
            }
        }
        let (flat_item_vertices, flat_item_indices) = self.collect_flat_item_model_geometry();
        if !flat_item_indices.is_empty() {
            if let Some(atlas) = &self.item_entity_atlas {
                self.draw_item_model_geometry(
                    &mut encoder,
                    &view,
                    &flat_item_vertices,
                    &flat_item_indices,
                    &atlas.bind_group,
                );
                pipeline_switches += 1;
                item_model_draw_calls += 1;
            }
        }

        let item_entity_vertices = self.collect_item_entity_vertices();
        if let Some(atlas) = &self.item_entity_atlas {
            if !item_entity_vertices.is_empty() {
                let item_entity_vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-item-entity-frame-vertices"),
                            contents: bytemuck::cast_slice(&item_entity_vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-native-item-entity-pass"),
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
                pass.set_pipeline(&self.item_entity_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, item_entity_vertex_buffer.slice(..));
                pass.draw(0..item_entity_vertices.len() as u32, 0..1);
                item_entity_draw_calls += 1;
            }
        }

        let particle_vertices = self.collect_particle_vertices();
        if let Some(atlas) = &self.particle_atlas {
            if !particle_vertices.is_empty() {
                let particle_vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-particle-frame-vertices"),
                            contents: bytemuck::cast_slice(&particle_vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-native-particle-pass"),
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
                pass.set_pipeline(&self.particle_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, particle_vertex_buffer.slice(..));
                pass.draw(0..particle_vertices.len() as u32, 0..1);
                particle_draw_calls += 1;
            }
        }

        if self.selection_outline.is_some()
            || self.entity_scene_outline.is_some()
            || self.entity_target_outline.is_some()
        {
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
            if let Some(outline) = &self.selection_outline {
                pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
                pass.draw(0..outline.vertex_count, 0..1);
                selection_draw_calls += 1;
            }
            if let Some(outline) = &self.entity_scene_outline {
                pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
                pass.draw(0..outline.vertex_count, 0..1);
                entity_scene_draw_calls += 1;
            }
            if let Some(outline) = &self.entity_target_outline {
                pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
                pass.draw(0..outline.vertex_count, 0..1);
                entity_target_draw_calls += 1;
            }
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

        // GUI 3D block-item icons: the hotbar's block items render as 3D models (vanilla inventory item
        // rendering) under the GUI ortho camera, on top of the 2D HUD, against a freshly-cleared depth
        // buffer so their faces sort within each slot. Block-light items sample the blocks atlas via the
        // GUI item bind group (the world camera's pass already finished, so reusing the depth target with
        // a clear is safe).
        {
            let gui_item_mesh = self.collect_hud_block_item_mesh();
            if !gui_item_mesh.indices.is_empty() {
                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-hud-block-item-vertices"),
                            contents: bytemuck::cast_slice(&gui_item_mesh.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let index_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-hud-block-item-indices"),
                            contents: bytemuck::cast_slice(&gui_item_mesh.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-native-hud-item-pass"),
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
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.item_model_pipeline);
                pass.set_bind_group(0, &self.gui_item_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gui_item_mesh.indices.len() as u32, 0, 0..1);
                pipeline_switches += 1;
                item_model_draw_calls += 1;
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
        self.counters.particle_draw_calls = particle_draw_calls;
        self.counters.item_entity_draw_calls = item_entity_draw_calls;
        self.counters.selection_draw_calls = selection_draw_calls;
        self.counters.entity_scene_draw_calls = entity_scene_draw_calls + entity_model_draw_calls;
        self.counters.entity_target_draw_calls = entity_target_draw_calls;
        self.counters.hud_draw_calls = hud_draw_calls;
        self.counters.draw_calls = opaque_draw_calls
            + cutout_draw_calls
            + translucent_draw_calls
            + block_destroy_overlay_draw_calls
            + entity_model_draw_calls
            + particle_draw_calls
            + item_entity_draw_calls
            + item_model_draw_calls
            + selection_draw_calls
            + entity_scene_draw_calls
            + entity_target_draw_calls
            + hud_draw_calls;
        self.counters.pipeline_switches = pipeline_switches;
        Ok(())
    }

    /// Draws one frame's merged item-model geometry: uploads the per-frame vertex + index buffers and
    /// issues a single indexed draw with the item-model pipeline against `bind_group` (the blocks atlas
    /// for block-items, or the item atlas for flat items). A `Load` pass over the shared color + depth
    /// targets, so item models depth-interact with the world drawn before them.
    fn draw_item_model_geometry(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        vertices: &[crate::item_models::ItemModelVertex],
        indices: &[u32],
        bind_group: &wgpu::BindGroup,
    ) {
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-item-model-frame-vertices"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-item-model-frame-indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bbb-native-item-model-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        pass.set_pipeline(&self.item_model_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }
}
