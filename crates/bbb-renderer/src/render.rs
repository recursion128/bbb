use std::path::Path;

use anyhow::Result;
use wgpu::util::DeviceExt;

use crate::Renderer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerrainOpaqueGroupLayer {
    Solid,
    Cutout,
}

const TERRAIN_OPAQUE_GROUP_LAYERS: &[TerrainOpaqueGroupLayer] = &[
    TerrainOpaqueGroupLayer::Solid,
    TerrainOpaqueGroupLayer::Cutout,
];
const ENTITY_OUTLINE_TARGET_PASS_LABEL: &str = "bbb-native-entity-outline-target-pass";
const ENTITY_OUTLINE_SOBEL_PASS_LABEL: &str = "bbb-native-entity-outline-sobel-pass";
const ENTITY_OUTLINE_BLUR_HORIZONTAL_PASS_LABEL: &str =
    "bbb-native-entity-outline-blur-horizontal-pass";
const ENTITY_OUTLINE_BLUR_VERTICAL_PASS_LABEL: &str =
    "bbb-native-entity-outline-blur-vertical-pass";
const ENTITY_OUTLINE_BLIT_PASS_LABEL: &str = "bbb-native-entity-outline-blit-pass";
const ENTITY_OUTLINE_COMPOSITE_PASS_LABEL: &str = "bbb-native-entity-outline-composite-pass";
const CLOUDS_PASS_LABEL: &str = "bbb-native-clouds-pass";
const CLOUDS_COMPOSITE_PASS_LABEL: &str = "bbb-native-clouds-composite-pass";
const MAIN_BLIT_PASS_LABEL: &str = "bbb-native-main-blit-pass";

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
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let main_view = &self.main_target.view;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-native-clear"),
            });

        let mut opaque_draw_calls = 0;
        let mut cutout_draw_calls = 0;
        let mut translucent_draw_calls = 0;
        let mut block_destroy_overlay_draw_calls = 0;
        let mut sky_draw_calls = 0;
        let mut entity_model_draw_calls = 0;
        let mut outline_composite_draw_calls = 0;
        let mut cloud_composite_draw_calls = 0;
        let mut main_blit_draw_calls = 0;
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
                label: Some("bbb-native-terrain-opaque-group-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: main_view,
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

            if self.sky_environment.end_sky_visible() {
                if let Some(end_sky_texture) = &self.end_sky_texture {
                    pass.set_pipeline(&self.end_sky_pipeline);
                    pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &end_sky_texture.bind_group, &[]);
                    pass.set_vertex_buffer(0, self.end_sky_mesh.vertex_buffer.slice(..));
                    pass.draw(0..self.end_sky_mesh.vertex_count, 0..1);
                    sky_draw_calls += 1;
                }
            } else if let Some(sky_disc) = &self.sky_disc {
                pass.set_pipeline(&self.sky_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_vertex_buffer(0, sky_disc.vertex_buffer.slice(..));
                pass.draw(0..sky_disc.vertex_count, 0..1);
                sky_draw_calls += 1;

                if let (Some(celestial_atlas), Some(celestials)) =
                    (&self.celestial_atlas, &self.sky_celestials)
                {
                    pass.set_pipeline(&self.celestial_pipeline);
                    pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &celestial_atlas.bind_group, &[]);
                    pass.set_vertex_buffer(0, celestials.vertex_buffer.slice(..));
                    pass.draw(0..celestials.vertex_count, 0..1);
                    sky_draw_calls += 1;
                }

                if let Some(stars) = &self.sky_stars {
                    pass.set_pipeline(&self.star_pipeline);
                    pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_vertex_buffer(0, stars.vertex_buffer.slice(..));
                    pass.draw(0..stars.vertex_count, 0..1);
                    sky_draw_calls += 1;
                }
            }

            // Vanilla 26.1 renders ChunkSectionLayerGroup.OPAQUE as SOLID then CUTOUT
            // before feature submissions; keep both terrain layers ahead of entity draws.
            for terrain_layer in TERRAIN_OPAQUE_GROUP_LAYERS {
                match terrain_layer {
                    TerrainOpaqueGroupLayer::Solid => {
                        if !self.terrain_opaque.is_empty() {
                            pass.set_pipeline(&self.terrain_pipeline);
                            pipeline_switches += 1;
                            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                            for mesh in &self.terrain_opaque {
                                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                pass.set_index_buffer(
                                    mesh.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                                opaque_draw_calls += 1;
                            }
                        }
                    }
                    TerrainOpaqueGroupLayer::Cutout => {
                        if !self.terrain_cutout.is_empty() {
                            pass.set_pipeline(&self.terrain_pipeline);
                            pipeline_switches += 1;
                            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                            for mesh in &self.terrain_cutout {
                                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                pass.set_index_buffer(
                                    mesh.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                                cutout_draw_calls += 1;
                            }
                        }
                    }
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

        if self.entity_model_texture_atlas.is_some()
            && (self.entity_model_outline_mesh.is_some()
                || self.entity_model_outline_cull_mesh.is_some())
        {
            let atlas = self
                .entity_model_texture_atlas
                .as_ref()
                .expect("outline meshes require the static entity atlas");
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(ENTITY_OUTLINE_TARGET_PASS_LABEL),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.entity_outline_target.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.entity_outline_target.depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                if let Some(mesh) = &self.entity_model_outline_mesh {
                    pass.set_pipeline(&self.entity_model_outline_pipeline);
                    pipeline_switches += 1;
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                    entity_model_draw_calls += 1;
                }
                if let Some(mesh) = &self.entity_model_outline_cull_mesh {
                    pass.set_pipeline(&self.entity_model_outline_cull_pipeline);
                    pipeline_switches += 1;
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                    entity_model_draw_calls += 1;
                }
            }

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(ENTITY_OUTLINE_SOBEL_PASS_LABEL),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.entity_outline_target.swap_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.entity_outline_sobel_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.bind_group, &[]);
                pass.draw(0..3, 0..1);
                outline_composite_draw_calls += 1;
            }

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(ENTITY_OUTLINE_BLUR_HORIZONTAL_PASS_LABEL),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.entity_outline_target.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.entity_outline_blur_horizontal_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.swap_linear_bind_group, &[]);
                pass.draw(0..3, 0..1);
                outline_composite_draw_calls += 1;
            }

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(ENTITY_OUTLINE_BLUR_VERTICAL_PASS_LABEL),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.entity_outline_target.swap_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.entity_outline_blur_vertical_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.linear_bind_group, &[]);
                pass.draw(0..3, 0..1);
                outline_composite_draw_calls += 1;
            }

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(ENTITY_OUTLINE_BLIT_PASS_LABEL),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.entity_outline_target.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.entity_outline_blit_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.swap_bind_group, &[]);
                pass.draw(0..3, 0..1);
                outline_composite_draw_calls += 1;
            }

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(ENTITY_OUTLINE_COMPOSITE_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: main_view,
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
            pass.set_pipeline(&self.entity_outline_composite_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.entity_outline_target.bind_group, &[]);
            pass.draw(0..3, 0..1);
            outline_composite_draw_calls += 1;
        }

        if let Some(clouds) = &self.clouds {
            if self.fog_environment.cloud_end > 0.0 {
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some(CLOUDS_PASS_LABEL),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.cloud_target.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.cloud_target.depth.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.cloud_pipeline);
                    pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &self.cloud_bind_group, &[]);
                    pass.set_vertex_buffer(0, clouds.vertex_buffer.slice(..));
                    pass.draw(0..clouds.vertex_count, 0..1);
                    sky_draw_calls += 1;
                }

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(CLOUDS_COMPOSITE_PASS_LABEL),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: main_view,
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
                pass.set_pipeline(&self.cloud_composite_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.cloud_target.bind_group, &[]);
                pass.draw(0..3, 0..1);
                cloud_composite_draw_calls += 1;
            }
        }

        if !self.terrain_translucent.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-terrain-translucent-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: main_view,
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
                    view: main_view,
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
                main_view,
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
                    main_view,
                    &map_vertices,
                    &map_indices,
                    &atlas.bind_group,
                );
                pipeline_switches += 1;
                item_model_draw_calls += 1;
            }
        }
        let (map_decoration_vertices, map_decoration_indices) =
            self.collect_item_frame_map_decoration_geometry();
        if !map_decoration_indices.is_empty() {
            if let Some(atlas) = &self.item_frame_map_decoration_atlas {
                self.draw_item_model_geometry(
                    &mut encoder,
                    main_view,
                    &map_decoration_vertices,
                    &map_decoration_indices,
                    &atlas.bind_group,
                );
                pipeline_switches += 1;
                item_model_draw_calls += 1;
            }
        }
        let (map_text_vertices, map_text_indices) = self.collect_item_frame_map_text_geometry();
        if !map_text_indices.is_empty() {
            if let Some(atlas) = &self.item_frame_map_text_font_atlas {
                self.draw_item_model_geometry(
                    &mut encoder,
                    main_view,
                    &map_text_vertices,
                    &map_text_indices,
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
                    main_view,
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
                        view: main_view,
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
                        view: main_view,
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
                    view: main_view,
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
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(MAIN_BLIT_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.main_blit_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.main_target.bind_group, &[]);
            pass.draw(0..3, 0..1);
            main_blit_draw_calls += 1;
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
                        view: &surface_view,
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
                        view: &surface_view,
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
        self.counters.sky_draw_calls = sky_draw_calls;
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
            + sky_draw_calls
            + entity_model_draw_calls
            + outline_composite_draw_calls
            + cloud_composite_draw_calls
            + main_blit_draw_calls
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

#[cfg(test)]
mod tests {
    use super::{TerrainOpaqueGroupLayer, TERRAIN_OPAQUE_GROUP_LAYERS};

    #[test]
    fn terrain_opaque_group_follows_vanilla_chunk_layer_order() {
        // Vanilla 26.1 ChunkSectionLayerGroup.OPAQUE is SOLID followed by CUTOUT;
        // LevelRenderer renders that group before feature submissions.
        assert_eq!(
            TERRAIN_OPAQUE_GROUP_LAYERS,
            &[
                TerrainOpaqueGroupLayer::Solid,
                TerrainOpaqueGroupLayer::Cutout,
            ]
        );
    }

    #[test]
    fn cloud_target_composites_after_main_and_outline_before_later_translucent_passes() {
        let source = include_str!("render.rs");
        let sky = source
            .find("pass.set_pipeline(&self.sky_pipeline)")
            .expect("sky pipeline is drawn");
        let terrain = source
            .find("for terrain_layer in TERRAIN_OPAQUE_GROUP_LAYERS")
            .expect("terrain opaque group is drawn");
        let outline_composite = source
            .find("label: Some(ENTITY_OUTLINE_COMPOSITE_PASS_LABEL)")
            .expect("entity outline composite pass label is used");
        let clouds = source
            .find("label: Some(CLOUDS_PASS_LABEL)")
            .expect("cloud pass label is used");
        let cloud_pipeline = source[clouds..]
            .find("pass.set_pipeline(&self.cloud_pipeline)")
            .map(|index| clouds + index)
            .expect("cloud pipeline is drawn in the cloud pass");
        let clouds_composite = source
            .find("label: Some(CLOUDS_COMPOSITE_PASS_LABEL)")
            .expect("cloud composite pass label is used");
        let translucent = source
            .find("label: Some(\"bbb-native-terrain-translucent-pass\")")
            .expect("terrain translucent pass label is used");

        assert!(sky < clouds, "clouds draw after the top sky disc");
        assert!(
            terrain < clouds,
            "clouds draw after the main terrain/entity pass"
        );
        assert!(
            outline_composite < clouds,
            "clouds draw after the entity outline post-chain like vanilla LevelRenderer"
        );
        assert!(
            clouds < cloud_pipeline && cloud_pipeline < clouds_composite,
            "cloud mesh draws into the dedicated clouds pass before compositing"
        );
        assert!(
            clouds_composite < translucent,
            "cloud target composites before later translucent world passes"
        );
        assert!(
            source[clouds..clouds_composite].contains("view: &self.cloud_target.view"),
            "cloud mesh writes the renderer-owned clouds color target"
        );
        assert!(
            source[clouds..clouds_composite].contains("view: &self.cloud_target.depth.view"),
            "cloud mesh writes the renderer-owned clouds depth target"
        );
        assert!(
            source[clouds_composite..translucent]
                .contains("pass.set_bind_group(0, &self.cloud_target.bind_group, &[])"),
            "cloud composite samples the renderer-owned clouds target"
        );
    }

    #[test]
    fn main_target_blits_to_surface_before_hud_and_screenshot_readback() {
        let source = include_str!("render.rs");
        let main_view = source
            .find("let main_view = &self.main_target.view")
            .expect("renderer-owned main target view is selected");
        let terrain_pass = source
            .find("label: Some(\"bbb-native-terrain-opaque-group-pass\")")
            .expect("main terrain pass label is used");
        let main_blit = source
            .find("label: Some(MAIN_BLIT_PASS_LABEL)")
            .expect("main blit pass label is used");
        let hud_pass = source
            .find("label: Some(\"bbb-native-hud-pass\")")
            .expect("hud pass label is used");
        let hud_item_pass = source
            .find("label: Some(\"bbb-native-hud-item-pass\")")
            .expect("hud item pass label is used");
        let screenshot_copy = source
            .find("prepare_screenshot_copy")
            .expect("screenshot copy still reads the presented frame");

        assert!(
            main_view < terrain_pass && terrain_pass < main_blit,
            "content passes draw to the renderer-owned main target before the final blit"
        );
        assert!(
            !source[..main_blit].contains("view: &surface_view"),
            "surface view is not a render target until the final blit pass"
        );
        assert!(
            source[terrain_pass..main_blit].contains("view: main_view"),
            "main content passes use the renderer-owned main target"
        );
        assert!(
            source[main_blit..hud_pass].contains("view: &surface_view"),
            "final blit writes the swapchain surface before HUD rendering"
        );
        assert!(
            source[main_blit..hud_pass]
                .contains("pass.set_bind_group(0, &self.main_target.bind_group, &[])"),
            "final blit samples the renderer-owned main target"
        );
        assert!(
            main_blit < hud_pass && hud_pass < hud_item_pass && hud_item_pass < screenshot_copy,
            "HUD and GUI item passes draw on the surface after final main-target blit"
        );
        assert!(
            source[hud_pass..screenshot_copy].contains("view: &surface_view"),
            "post-blit HUD passes target the swapchain surface"
        );
    }

    #[test]
    fn cloud_presentation_binds_cloud_offset_uniform() {
        let source = include_str!("render.rs");
        let clouds = source
            .find("pass.set_pipeline(&self.cloud_pipeline)")
            .expect("cloud pipeline is drawn");
        let cloud_uniform = source
            .find("pass.set_bind_group(1, &self.cloud_bind_group, &[])")
            .expect("cloud offset bind group is bound");
        let cloud_draw = source[cloud_uniform..]
            .find("pass.draw(0..clouds.vertex_count, 0..1)")
            .map(|index| cloud_uniform + index)
            .expect("cloud mesh is drawn");

        assert!(
            clouds < cloud_uniform && cloud_uniform < cloud_draw,
            "cloud offset uniform is bound after selecting the cloud pipeline and before drawing"
        );
    }

    #[test]
    fn entity_outline_target_composites_before_later_world_passes() {
        // Vanilla LevelRenderer adds the entity_outline post chain immediately after the main pass
        // and before later target/post-chain work. Keep bbb's target write, post-chain, and
        // final composite before the remaining standalone world passes.
        let source = include_str!("render.rs");
        let target = source
            .find("label: Some(ENTITY_OUTLINE_TARGET_PASS_LABEL)")
            .expect("entity outline target pass label is used");
        let sobel = source
            .find("label: Some(ENTITY_OUTLINE_SOBEL_PASS_LABEL)")
            .expect("entity outline sobel pass label is used");
        let blur_horizontal = source
            .find("label: Some(ENTITY_OUTLINE_BLUR_HORIZONTAL_PASS_LABEL)")
            .expect("entity outline horizontal blur pass label is used");
        let blur_vertical = source
            .find("label: Some(ENTITY_OUTLINE_BLUR_VERTICAL_PASS_LABEL)")
            .expect("entity outline vertical blur pass label is used");
        let blit = source
            .find("label: Some(ENTITY_OUTLINE_BLIT_PASS_LABEL)")
            .expect("entity outline blit pass label is used");
        let composite = source
            .find("label: Some(ENTITY_OUTLINE_COMPOSITE_PASS_LABEL)")
            .expect("entity outline composite pass label is used");
        let translucent = source
            .find("label: Some(\"bbb-native-terrain-translucent-pass\")")
            .expect("terrain translucent pass label is used");
        assert!(
            target < sobel
                && sobel < blur_horizontal
                && blur_horizontal < blur_vertical
                && blur_vertical < blit
                && blit < composite,
            "outline target and post-chain passes follow vanilla entity_outline.json order"
        );
        assert!(
            composite < translucent,
            "outline composite stays before later standalone world passes"
        );
    }
}
