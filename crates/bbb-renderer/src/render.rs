use std::path::Path;

use anyhow::Result;
use wgpu::util::DeviceExt;

use crate::{
    lightmap::write_lightmap_uniform,
    weather::{build_lightning_mesh, build_weather_mesh},
    Renderer,
};

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
const ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL: &str = "bbb-native-entity-translucent-feature-pass";
const TRANSLUCENT_TARGET_PASS_LABEL: &str = "bbb-native-translucent-target-pass";
const ITEM_ENTITY_TARGET_PASS_LABEL: &str = "bbb-native-item-entity-target-pass";
const PARTICLE_TARGET_PASS_LABEL: &str = "bbb-native-particle-target-pass";
const WEATHER_TARGET_PASS_LABEL: &str = "bbb-native-weather-target-pass";
const LIGHTMAP_PASS_LABEL: &str = "bbb-native-lightmap-pass";
const TRANSPARENCY_COMBINE_PASS_LABEL: &str = "bbb-native-transparency-combine-pass";

struct ItemModelFrameBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

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
        let translucent_view = &self.translucent_target.view;
        let item_entity_view = &self.item_entity_target.view;
        let particle_view = &self.particle_target.view;
        let weather_view = &self.weather_target.view;
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
        let mut transparency_combine_draw_calls = 0;
        let mut particle_draw_calls = 0;
        let mut weather_draw_calls = 0;
        let mut item_entity_draw_calls = 0;
        let mut item_model_draw_calls = 0;
        let mut selection_draw_calls = 0;
        let mut entity_scene_draw_calls = 0;
        let mut entity_target_draw_calls = 0;
        let mut hud_draw_calls = 0;
        let mut lightmap_draw_calls = 0;
        let mut pipeline_switches = 0;
        write_lightmap_uniform(
            &self.queue,
            &self.lightmap.uniform_buffer,
            self.lightmap_environment,
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(LIGHTMAP_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.lightmap.view,
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
            pass.set_pipeline(&self.lightmap_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.lightmap.bind_group, &[]);
            pass.draw(0..3, 0..1);
            lightmap_draw_calls += 1;
        }
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
                            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
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
                            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
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
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
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
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
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
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
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
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                entity_model_draw_calls += 1;
            }
        }

        // Vanilla solid item features are part of FeatureRenderDispatcher.renderSolidFeatures()
        // inside the main pass: after model/model-part features and before target depth copies,
        // translucent terrain, entity-outline post-chain work, and clouds. GUI item icons use
        // the later HUD item pass instead.
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

        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::ImageCopyTexture {
                texture: &self.translucent_target.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::Extent3d {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
                depth_or_array_layers: 1,
            },
        );

        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::ImageCopyTexture {
                texture: &self.item_entity_target.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::Extent3d {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
                depth_or_array_layers: 1,
            },
        );

        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::ImageCopyTexture {
                texture: &self.particle_target.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::Extent3d {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
                depth_or_array_layers: 1,
            },
        );

        if self.has_entity_translucent_features() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL),
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
            self.draw_entity_translucent_features(
                &mut pass,
                &mut pipeline_switches,
                &mut entity_model_draw_calls,
            );
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

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(TRANSLUCENT_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: translucent_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.translucent_target.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            if !self.terrain_translucent.is_empty() {
                pass.set_pipeline(&self.terrain_translucent_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                for mesh in &self.terrain_translucent {
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                    translucent_draw_calls += 1;
                }
            }
        }

        let item_entity_vertices = self.collect_item_entity_vertices();
        let item_entity_vertex_buffer =
            if self.item_entity_atlas.is_some() && !item_entity_vertices.is_empty() {
                Some(
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-item-entity-frame-vertices"),
                            contents: bytemuck::cast_slice(&item_entity_vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            } else {
                None
            };
        let (block_item_translucent_vertices, block_item_translucent_indices) =
            self.collect_block_item_model_translucent_geometry();
        let block_item_translucent_buffers = self.create_item_model_frame_buffers(
            &block_item_translucent_vertices,
            &block_item_translucent_indices,
        );
        let (flat_item_translucent_vertices, flat_item_translucent_indices) =
            self.collect_flat_item_model_translucent_geometry();
        let flat_item_translucent_buffers = self.create_item_model_frame_buffers(
            &flat_item_translucent_vertices,
            &flat_item_translucent_indices,
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(ITEM_ENTITY_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: item_entity_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.item_entity_target.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.draw_entity_item_entity_target_features(
                &mut pass,
                &mut pipeline_switches,
                &mut entity_model_draw_calls,
            );
            if let Some(buffers) = &block_item_translucent_buffers {
                self.draw_item_model_frame_buffers(
                    &mut pass,
                    &self.item_model_translucent_pipeline,
                    buffers,
                    &self.terrain_bind_group,
                );
                pipeline_switches += 1;
                item_model_draw_calls += 1;
            }
            if let (Some(atlas), Some(buffers)) =
                (&self.item_entity_atlas, &flat_item_translucent_buffers)
            {
                self.draw_item_model_frame_buffers(
                    &mut pass,
                    &self.item_model_translucent_pipeline,
                    buffers,
                    &atlas.bind_group,
                );
                pipeline_switches += 1;
                item_model_draw_calls += 1;
            }
            if let (Some(atlas), Some(vertex_buffer)) =
                (&self.item_entity_atlas, &item_entity_vertex_buffer)
            {
                pass.set_pipeline(&self.item_entity_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..item_entity_vertices.len() as u32, 0..1);
                item_entity_draw_calls += 1;
            }
            if self.selection_outline.is_some()
                || self.entity_scene_outline.is_some()
                || self.entity_target_outline.is_some()
            {
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
        }

        let particle_vertices = self.collect_particle_vertices();
        let particle_vertex_buffer =
            if self.particle_atlas.is_some() && !particle_vertices.is_empty() {
                Some(
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-particle-frame-vertices"),
                            contents: bytemuck::cast_slice(&particle_vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            } else {
                None
            };
        let weather_mesh = build_weather_mesh(&self.weather_render_state);
        let weather_buffers = weather_mesh
            .as_ref()
            .filter(|mesh| {
                (!mesh.rain_indices.is_empty() && self.weather_rain_texture.is_some())
                    || (!mesh.snow_indices.is_empty() && self.weather_snow_texture.is_some())
            })
            .map(|mesh| {
                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-weather-frame-vertices"),
                            contents: bytemuck::cast_slice(&mesh.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let index_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-weather-frame-indices"),
                            contents: bytemuck::cast_slice(&mesh.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                (vertex_buffer, index_buffer)
            });
        let lightning_mesh = build_lightning_mesh(&self.weather_render_state);
        let lightning_buffers = lightning_mesh.as_ref().map(|mesh| {
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bbb-lightning-frame-vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bbb-lightning-frame-indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
            (vertex_buffer, index_buffer)
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(PARTICLE_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: particle_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.particle_target.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            if let (Some(atlas), Some(vertex_buffer)) =
                (&self.particle_atlas, &particle_vertex_buffer)
            {
                pass.set_pipeline(&self.particle_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..particle_vertices.len() as u32, 0..1);
                particle_draw_calls += 1;
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
            }
        }

        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::ImageCopyTexture {
                texture: &self.weather_target.depth._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::Extent3d {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
                depth_or_array_layers: 1,
            },
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(WEATHER_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: weather_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.weather_target.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            if let (Some(mesh), Some((vertex_buffer, index_buffer))) =
                (&lightning_mesh, &lightning_buffers)
            {
                pass.set_pipeline(&self.lightning_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                weather_draw_calls += 1;
            }
            if let (Some(mesh), Some((vertex_buffer, index_buffer))) =
                (&weather_mesh, &weather_buffers)
            {
                pass.set_pipeline(&self.weather_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                if !mesh.rain_indices.is_empty() {
                    if let Some(texture) = &self.weather_rain_texture {
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.draw_indexed(mesh.rain_indices.clone(), 0, 0..1);
                        weather_draw_calls += 1;
                    }
                }
                if !mesh.snow_indices.is_empty() {
                    if let Some(texture) = &self.weather_snow_texture {
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.draw_indexed(mesh.snow_indices.clone(), 0, 0..1);
                        weather_draw_calls += 1;
                    }
                }
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(TRANSPARENCY_COMBINE_PASS_LABEL),
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
            pass.set_pipeline(&self.transparency_combine_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.transparency_combine_bind_group.bind_group, &[]);
            pass.draw(0..3, 0..1);
            transparency_combine_draw_calls += 1;
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
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
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
        self.counters.weather_draw_calls = weather_draw_calls;
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
            + transparency_combine_draw_calls
            + particle_draw_calls
            + weather_draw_calls
            + item_entity_draw_calls
            + item_model_draw_calls
            + selection_draw_calls
            + entity_scene_draw_calls
            + entity_target_draw_calls
            + hud_draw_calls
            + lightmap_draw_calls;
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
        let Some(buffers) = self.create_item_model_frame_buffers(vertices, indices) else {
            return;
        };
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
        self.draw_item_model_frame_buffers(
            &mut pass,
            &self.item_model_pipeline,
            &buffers,
            bind_group,
        );
    }

    fn create_item_model_frame_buffers(
        &self,
        vertices: &[crate::item_models::ItemModelVertex],
        indices: &[u32],
    ) -> Option<ItemModelFrameBuffers> {
        if indices.is_empty() {
            return None;
        }
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
        Some(ItemModelFrameBuffers {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        })
    }

    fn draw_item_model_frame_buffers<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a wgpu::RenderPipeline,
        buffers: &'a ItemModelFrameBuffers,
        bind_group: &'a wgpu::BindGroup,
    ) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
        pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
        pass.set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..buffers.index_count, 0, 0..1);
    }

    fn has_entity_translucent_features(&self) -> bool {
        (self.entity_model_texture_atlas.is_some()
            && (self.entity_model_translucent_mesh.is_some()
                || self.entity_model_eyes_mesh.is_some()
                || self.entity_model_scroll_mesh.is_some()
                || self.entity_model_scroll_additive_mesh.is_some()))
            || (self.entity_dynamic_player_skin_atlas.is_some()
                && self.entity_dynamic_player_skin_translucent_mesh.is_some())
            || (self.entity_dynamic_player_texture_atlas.is_some()
                && self
                    .entity_dynamic_player_texture_translucent_mesh
                    .is_some())
    }

    fn draw_entity_item_entity_target_features<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_model_item_entity_translucent_mesh,
            &self.entity_model_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_dynamic_player_skin_item_entity_translucent_mesh,
            &self.entity_dynamic_player_skin_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_dynamic_player_texture_item_entity_translucent_mesh,
            &self.entity_dynamic_player_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
    }

    fn draw_entity_translucent_features<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_model_translucent_mesh,
            &self.entity_model_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_dynamic_player_skin_translucent_mesh,
            &self.entity_dynamic_player_skin_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_dynamic_player_texture_translucent_mesh,
            &self.entity_dynamic_player_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_model_eyes_mesh,
            &self.entity_model_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_eyes_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_model_scroll_mesh,
            &self.entity_model_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_scroll_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_model_scroll_additive_mesh,
            &self.entity_model_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_scroll_additive_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TerrainOpaqueGroupLayer, TERRAIN_OPAQUE_GROUP_LAYERS};

    fn depth_copy_to(source: &str, target_depth_texture: &str) -> usize {
        let target_depth = source
            .find(target_depth_texture)
            .unwrap_or_else(|| panic!("{target_depth_texture} depth copy destination"));
        source[..target_depth]
            .rfind("encoder.copy_texture_to_texture")
            .expect("main depth copy starts before target depth destination")
    }

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
    fn lightmap_pass_updates_texture_before_world_passes() {
        let source = include_str!("render.rs");
        let lightmap_write = source
            .find("write_lightmap_uniform(")
            .expect("lightmap uniform is updated before rendering");
        let lightmap = source
            .find("label: Some(LIGHTMAP_PASS_LABEL)")
            .expect("lightmap pass label is used");
        let lightmap_pipeline = source[lightmap..]
            .find("pass.set_pipeline(&self.lightmap_pipeline)")
            .map(|index| lightmap + index)
            .expect("lightmap pipeline is selected");
        let lightmap_draw = source[lightmap_pipeline..]
            .find("pass.draw(0..3, 0..1)")
            .map(|index| lightmap_pipeline + index)
            .expect("lightmap pass draws the vanilla screen triangle");
        let terrain_pass = source
            .find("label: Some(\"bbb-native-terrain-opaque-group-pass\")")
            .expect("main terrain pass label is used");

        assert!(
            lightmap_write < lightmap
                && lightmap < lightmap_pipeline
                && lightmap_pipeline < lightmap_draw
                && lightmap_draw < terrain_pass,
            "vanilla GameRenderer updates Lightmap before world rendering samples the level lightmap"
        );
        assert!(
            source[lightmap..terrain_pass].contains("view: &self.lightmap.view"),
            "lightmap pass writes the renderer-owned LightTexture"
        );
        assert!(
            source[lightmap..terrain_pass]
                .contains("pass.set_bind_group(0, &self.lightmap.bind_group, &[])"),
            "lightmap pass binds the standalone LightmapInfo uniform"
        );
    }

    #[test]
    fn terrain_draws_sample_dynamic_lightmap_texture() {
        let source = include_str!("render.rs");
        let terrain = source
            .find("pass.set_pipeline(&self.terrain_pipeline)")
            .expect("opaque terrain pipeline is selected");
        let terrain_atlas = source[terrain..]
            .find("pass.set_bind_group(0, &self.terrain_bind_group, &[])")
            .map(|index| terrain + index)
            .expect("terrain bind group is bound");
        let lightmap = source[terrain_atlas..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| terrain_atlas + index)
            .expect("terrain lightmap sampler bind group is bound");
        let draw = source[lightmap..]
            .find("pass.draw_indexed")
            .map(|index| lightmap + index)
            .expect("terrain draw follows bind groups");
        let translucent = source
            .find("pass.set_pipeline(&self.terrain_translucent_pipeline)")
            .expect("translucent terrain pipeline is selected");
        let translucent_lightmap = source[translucent..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| translucent + index)
            .expect("translucent terrain lightmap sampler bind group is bound");

        assert!(
            terrain < terrain_atlas && terrain_atlas < lightmap && lightmap < draw,
            "terrain samples the renderer-owned LightTexture before opaque draws"
        );
        assert!(
            translucent < translucent_lightmap,
            "translucent terrain samples the same dynamic LightTexture"
        );
    }

    #[test]
    fn lit_entity_draws_sample_dynamic_lightmap_texture() {
        let source = include_str!("render.rs");
        for (pipeline, bind_group, label) in [
            (
                "pass.set_pipeline(&self.entity_model_pipeline)",
                "pass.set_bind_group(0, &self.terrain_bind_group, &[])",
                "colored entity fallback",
            ),
            (
                "pass.set_pipeline(&self.entity_model_textured_pipeline)",
                "pass.set_bind_group(0, &atlas.bind_group, &[])",
                "textured entity",
            ),
            (
                "pass.set_pipeline(&self.entity_model_translucent_pipeline)",
                "pass.set_bind_group(0, &atlas.bind_group, &[])",
                "translucent entity",
            ),
            (
                "pass.set_pipeline(&self.entity_model_scroll_pipeline)",
                "pass.set_bind_group(0, &atlas.bind_group, &[])",
                "breezeWind lit scroll entity",
            ),
        ] {
            let pipeline = source
                .find(pipeline)
                .unwrap_or_else(|| panic!("{label} pipeline"));
            let atlas = source[pipeline..]
                .find(bind_group)
                .map(|index| pipeline + index)
                .unwrap_or_else(|| panic!("{label} texture/camera bind group"));
            let lightmap = source[atlas..]
                .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
                .map(|index| atlas + index)
                .unwrap_or_else(|| panic!("{label} lightmap bind group"));
            let draw = source[lightmap..]
                .find("pass.draw_indexed")
                .map(|index| lightmap + index)
                .unwrap_or_else(|| panic!("{label} draw"));

            assert!(
                pipeline < atlas && atlas < lightmap && lightmap < draw,
                "{label} binds the renderer-owned LightTexture before drawing"
            );
        }

        let eyes = source
            .find("pass.set_pipeline(&self.entity_model_eyes_pipeline)")
            .expect("eyes pipeline");
        let eyes_draw = source[eyes..]
            .find("pass.draw_indexed")
            .map(|index| eyes + index)
            .expect("eyes draw");
        assert!(
            !source[eyes..eyes_draw]
                .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])"),
            "emissive eyes do not explicitly sample the dynamic lightmap"
        );
    }

    #[test]
    fn item_model_draws_sample_dynamic_lightmap_texture() {
        let source = include_str!("render.rs");
        let helper = source
            .find("fn draw_item_model_geometry(")
            .expect("item-model helper is present");
        let world_pipeline = source[helper..]
            .find("pass.set_pipeline(&self.item_model_pipeline)")
            .map(|index| helper + index)
            .expect("world item-model pipeline is selected");
        let world_atlas = source[world_pipeline..]
            .find("pass.set_bind_group(0, bind_group, &[])")
            .map(|index| world_pipeline + index)
            .expect("world item-model atlas bind group is bound");
        let world_lightmap = source[world_atlas..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| world_atlas + index)
            .expect("world item-model lightmap bind group is bound");
        let world_draw = source[world_lightmap..]
            .find("pass.draw_indexed")
            .map(|index| world_lightmap + index)
            .expect("world item-model draw follows bind groups");

        assert!(
            world_pipeline < world_atlas
                && world_atlas < world_lightmap
                && world_lightmap < world_draw,
            "world item-model draws bind the renderer-owned LightTexture before drawing"
        );

        let hud_pass = source
            .find("label: Some(\"bbb-native-hud-item-pass\")")
            .expect("HUD item-model pass is present");
        let hud_pipeline = source[hud_pass..]
            .find("pass.set_pipeline(&self.item_model_pipeline)")
            .map(|index| hud_pass + index)
            .expect("HUD item-model pipeline is selected");
        let hud_atlas = source[hud_pipeline..]
            .find("pass.set_bind_group(0, &self.gui_item_bind_group, &[])")
            .map(|index| hud_pipeline + index)
            .expect("HUD item-model atlas bind group is bound");
        let hud_lightmap = source[hud_atlas..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| hud_atlas + index)
            .expect("HUD item-model lightmap bind group is bound");
        let hud_draw = source[hud_lightmap..]
            .find("pass.draw_indexed")
            .map(|index| hud_lightmap + index)
            .expect("HUD item-model draw follows bind groups");

        assert!(
            hud_pipeline < hud_atlas && hud_atlas < hud_lightmap && hud_lightmap < hud_draw,
            "HUD 3D block item draws bind the renderer-owned LightTexture before drawing"
        );
    }

    #[test]
    fn world_item_models_draw_before_target_depth_copies_and_hud_items_stay_late() {
        let source = include_str!("render.rs");
        let world_item_models = source
            .find("let (block_item_vertices, block_item_indices)")
            .expect("solid world item-model collection is present");
        let world_item_draw = source[world_item_models..]
            .find("self.draw_item_model_geometry(")
            .map(|index| world_item_models + index)
            .expect("solid world item-model draw helper is called");
        let copy_translucent =
            depth_copy_to(source, "texture: &self.translucent_target.depth._texture");
        let outline_composite = source
            .find("label: Some(ENTITY_OUTLINE_COMPOSITE_PASS_LABEL)")
            .expect("entity outline composite pass label is used");
        let clouds = source
            .find("label: Some(CLOUDS_PASS_LABEL)")
            .expect("cloud pass label is used");
        let entity_translucent_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let translucent_target = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");
        let particle_target = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");
        let hud_item = source
            .find("label: Some(\"bbb-native-hud-item-pass\")")
            .expect("HUD item pass label is used");

        assert!(
            world_item_models < world_item_draw
                && world_item_draw < copy_translucent
                && copy_translucent < entity_translucent_features
                && entity_translucent_features < translucent_target
                && translucent_target < particle_target
                && particle_target < outline_composite
                && outline_composite < clouds,
            "vanilla ItemFeatureRenderer solid output contributes to main depth before later target-backed main pass work and outline/cloud post passes"
        );
        assert!(
            source[world_item_draw..copy_translucent].contains("main_view"),
            "world item models write the renderer-owned main color target before depth copies"
        );
        assert!(
            combine < hud_item,
            "GUI item icons remain a post-world HUD pass rather than joining world item features"
        );
    }

    #[test]
    fn translucent_item_models_draw_inside_item_entity_target_before_billboards_and_particles() {
        let source = include_str!("render.rs");
        let solid_collect = source
            .find("let (block_item_vertices, block_item_indices)")
            .expect("solid world item-model collection is present");
        let copy_item_entity =
            depth_copy_to(source, "texture: &self.item_entity_target.depth._texture");
        let block_collect = source
            .find("let (block_item_translucent_vertices, block_item_translucent_indices)")
            .expect("translucent block item-model collection is present");
        let flat_collect = source
            .find("let (flat_item_translucent_vertices, flat_item_translucent_indices)")
            .expect("translucent flat item-model collection is present");
        let target = source
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .expect("item-entity target pass label is used");
        let block_draw = source[target..]
            .find("&self.item_model_translucent_pipeline")
            .map(|index| target + index)
            .expect("translucent block item-model pipeline is drawn in item_entity target");
        let block_bind_group = source[block_draw..]
            .find("&self.terrain_bind_group")
            .map(|index| block_draw + index)
            .expect("translucent block item models bind the blocks atlas");
        let flat_draw = source[block_bind_group..]
            .find("&self.item_model_translucent_pipeline")
            .map(|index| block_bind_group + index)
            .expect("translucent flat item-model pipeline is drawn in item_entity target");
        let flat_bind_group = source[flat_draw..]
            .find("&atlas.bind_group")
            .map(|index| flat_draw + index)
            .expect("translucent flat item models bind the item atlas");
        let billboards = source[target..]
            .find("pass.set_pipeline(&self.item_entity_pipeline)")
            .map(|index| target + index)
            .expect("item-entity billboards are drawn in item_entity target");
        let particle = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");

        assert!(
            solid_collect < copy_item_entity
                && copy_item_entity < block_collect
                && block_collect < flat_collect
                && flat_collect < target
                && target < block_draw
                && block_bind_group < flat_draw
                && flat_bind_group < billboards
                && billboards < particle,
            "vanilla ItemFeatureRenderer translucent item submits draw through item_entity target before billboard items and particles"
        );
    }

    #[test]
    fn cloud_target_feeds_transparency_combine_after_world_passes() {
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
        let translucent = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");
        let item_entity_target = source
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .expect("item-entity target pass label is used");
        let particle_target = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let weather_target = source
            .find("label: Some(WEATHER_TARGET_PASS_LABEL)")
            .expect("weather target pass label is used");
        let entity_translucent_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let block_destroy = source
            .find("label: Some(\"bbb-native-block-destroy-overlay-pass\")")
            .expect("block destroy overlay pass label is used");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");
        let hud = source
            .find("label: Some(\"bbb-native-hud-pass\")")
            .expect("HUD pass label is used");

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
            entity_translucent_features < translucent
                && entity_translucent_features < block_destroy
                && block_destroy < translucent
                && translucent < item_entity_target
                && item_entity_target < particle_target
                && particle_target < outline_composite
                && outline_composite < clouds
                && particle_target < clouds
                && clouds < cloud_pipeline
                && cloud_pipeline < weather_target
                && particle_target < weather_target
                && weather_target < combine
                && combine < hud,
            "vanilla frame graph runs cloud pass after the main pass/particles and before weather and transparency combine"
        );
        assert!(
            source[clouds..weather_target].contains("view: &self.cloud_target.view"),
            "cloud mesh writes the renderer-owned clouds color target"
        );
        assert!(
            source[clouds..weather_target].contains("view: &self.cloud_target.depth.view"),
            "cloud mesh writes the renderer-owned clouds depth target"
        );
        assert!(
            source[translucent..combine].contains("view: translucent_view"),
            "translucent terrain writes the renderer-owned translucent color target"
        );
        assert!(
            source[translucent..combine].contains("view: &self.translucent_target.depth.view"),
            "translucent terrain writes the renderer-owned translucent depth target"
        );
        assert!(
            source[item_entity_target..combine].contains("view: item_entity_view"),
            "item-entity geometry writes the renderer-owned item_entity color target"
        );
        assert!(
            source[item_entity_target..combine]
                .contains("view: &self.item_entity_target.depth.view"),
            "item-entity geometry writes the renderer-owned item_entity depth target"
        );
        assert!(
            source[particle_target..combine].contains("view: particle_view"),
            "particle geometry writes the renderer-owned particles color target"
        );
        assert!(
            source[particle_target..combine].contains("view: &self.particle_target.depth.view"),
            "particle geometry writes the renderer-owned particles depth target"
        );
        assert!(
            source[weather_target..combine].contains("view: weather_view"),
            "weather pass clears the renderer-owned weather color target"
        );
        assert!(
            source[weather_target..combine].contains("view: &self.weather_target.depth.view"),
            "weather pass owns the renderer-owned weather depth target"
        );
        assert!(
            source[combine..hud].contains(
                "pass.set_bind_group(0, &self.transparency_combine_bind_group.bind_group, &[])"
            ),
            "transparency combine samples the renderer-owned main/translucent/item/cloud targets"
        );
    }

    #[test]
    fn translucent_target_copies_main_depth_and_clears_color_before_draws() {
        let source = include_str!("render.rs");
        let copy_depth = depth_copy_to(source, "texture: &self.translucent_target.depth._texture");
        let target = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");
        let terrain_pipeline = source[target..]
            .find("pass.set_pipeline(&self.terrain_translucent_pipeline)")
            .map(|index| target + index)
            .expect("terrain translucent pipeline is drawn into the target");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        assert!(
            copy_depth < target && target < terrain_pipeline && terrain_pipeline < combine,
            "vanilla LevelRenderer.copyDepthFrom(main) happens before translucent target draws and post/transparency consumes it"
        );
        assert!(
            source[copy_depth..target].contains("texture: &self.depth._texture")
                && source[copy_depth..target]
                    .contains("texture: &self.translucent_target.depth._texture"),
            "translucent target depth is copied from the renderer-owned main depth texture"
        );
        assert!(
            source[target..terrain_pipeline].contains("load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)"),
            "translucent target color is cleared every frame so missing translucent meshes do not reuse stale color"
        );
    }

    #[test]
    fn entity_translucent_features_draw_after_depth_copies_and_before_translucent_terrain() {
        let source = include_str!("render.rs");
        let copy_translucent =
            depth_copy_to(source, "texture: &self.translucent_target.depth._texture");
        let copy_item_entity =
            depth_copy_to(source, "texture: &self.item_entity_target.depth._texture");
        let copy_particles = depth_copy_to(source, "texture: &self.particle_target.depth._texture");
        let entity_pass = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let draw_features = source[entity_pass..]
            .find("self.draw_entity_translucent_features(")
            .map(|index| entity_pass + index)
            .expect("entity translucent feature helper is called from the pass");
        let translucent_target = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");

        assert!(
            copy_translucent < copy_item_entity
                && copy_item_entity < copy_particles
                && copy_particles < entity_pass
                && entity_pass < draw_features
                && draw_features < translucent_target,
            "vanilla LevelRenderer copies target depths before renderTranslucentFeatures, which draw before translucent terrain"
        );
        assert!(
            source[entity_pass..translucent_target].contains("view: main_view"),
            "entity translucent features write the renderer-owned main color target"
        );
        assert!(
            source[entity_pass..translucent_target].contains("view: &self.depth.view"),
            "entity translucent features depth-test against the renderer-owned main depth target"
        );
        let helper = source
            .find("fn draw_entity_translucent_features")
            .expect("entity translucent feature helper is present");
        for pipeline in [
            "pass.set_pipeline(&self.entity_model_translucent_pipeline)",
            "pass.set_pipeline(&self.entity_model_eyes_pipeline)",
            "pass.set_pipeline(&self.entity_model_scroll_pipeline)",
            "pass.set_pipeline(&self.entity_model_scroll_additive_pipeline)",
        ] {
            assert!(
                source[helper..].contains(pipeline),
                "{pipeline} is emitted through the translucent feature helper"
            );
        }
    }

    #[test]
    fn block_destroy_overlays_draw_in_translucent_feature_phase_before_translucent_terrain() {
        let source = include_str!("render.rs");
        let entity_pass = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let block_destroy = source
            .find("label: Some(\"bbb-native-block-destroy-overlay-pass\")")
            .expect("block destroy overlay pass label is used");
        let block_destroy_pipeline = source[block_destroy..]
            .find("pass.set_pipeline(&self.block_destroy_pipeline)")
            .map(|index| block_destroy + index)
            .expect("block destroy pipeline is selected");
        let translucent_target = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");

        assert!(
            entity_pass < block_destroy
                && block_destroy < block_destroy_pipeline
                && block_destroy_pipeline < translucent_target,
            "vanilla crumblingBufferSource.endBatch runs during translucent features before translucent terrain"
        );
        assert!(
            source[block_destroy..translucent_target].contains("view: main_view"),
            "block destroy overlays write the renderer-owned main color target"
        );
        assert!(
            source[block_destroy..translucent_target].contains("view: &self.depth.view"),
            "block destroy overlays depth-test against the renderer-owned main depth target"
        );
    }

    #[test]
    fn item_entity_target_copies_main_depth_and_collects_item_and_line_draws_before_particles() {
        let source = include_str!("render.rs");
        let entity_translucent_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let target = source
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .expect("item-entity target pass label is used");
        let copy_depth = depth_copy_to(source, "texture: &self.item_entity_target.depth._texture");
        let entity_item_target_draw = source[target..]
            .find("self.draw_entity_item_entity_target_features(")
            .map(|index| target + index)
            .expect("entity ITEM_ENTITY_TARGET features are drawn into item_entity target");
        let item_pipeline = source[target..]
            .find("pass.set_pipeline(&self.item_entity_pipeline)")
            .map(|index| target + index)
            .expect("item-entity pipeline is drawn into the target");
        let item_atlas = source[item_pipeline..]
            .find("pass.set_bind_group(0, &atlas.bind_group, &[])")
            .map(|index| item_pipeline + index)
            .expect("item-entity atlas bind group is bound before draw");
        let item_lightmap = source[item_atlas..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| item_atlas + index)
            .expect("item-entity lightmap bind group is bound before draw");
        let selection_pipeline = source[target..]
            .find("pass.set_pipeline(&self.selection_pipeline)")
            .map(|index| target + index)
            .expect("selection line pipeline is drawn into the item-entity target");
        let particle = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        assert!(
            copy_depth < entity_translucent_features
                && entity_translucent_features < target
                && target < entity_item_target_draw
                && entity_item_target_draw < item_pipeline
                && item_lightmap < selection_pipeline
                && selection_pipeline < particle
                && particle < combine,
            "item_entity target copies main depth, collects item/line draws, then particles and transparency combine run later"
        );
        assert!(
            item_pipeline < item_atlas && item_atlas < item_lightmap,
            "item-entity billboards bind the renderer-owned LightTexture before drawing"
        );
        assert!(
            source[copy_depth..entity_translucent_features]
                .contains("texture: &self.depth._texture")
                && source[copy_depth..entity_translucent_features]
                    .contains("texture: &self.item_entity_target.depth._texture"),
            "item_entity target depth is copied from the renderer-owned main depth texture"
        );
        assert!(
            source[target..item_pipeline]
                .contains("load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)"),
            "item_entity target color is cleared every frame so missing item draws do not reuse stale color"
        );
        assert!(
            source[target..particle].contains("view: item_entity_view")
                && source[target..particle].contains("view: &self.item_entity_target.depth.view"),
            "item and line geometry render into the item_entity color/depth target"
        );
    }

    #[test]
    fn entity_translucent_cull_item_target_draws_into_item_entity_target_not_main_features() {
        let source = include_str!("render.rs");
        let target = source
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .expect("item-entity target pass label is used");
        let item_target_draw = source[target..]
            .find("self.draw_entity_item_entity_target_features(")
            .map(|index| target + index)
            .expect("entity item-target helper is called from item_entity target pass");
        let particle = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let main_helper = source
            .find("fn draw_entity_translucent_features")
            .expect("main translucent feature helper is present");
        let item_helper = source
            .find("fn draw_entity_item_entity_target_features")
            .expect("item_entity target feature helper is present");
        let tests_mod = source
            .find("#[cfg(test)]")
            .expect("render tests module is present");

        assert!(
            target < item_target_draw && item_target_draw < particle,
            "vanilla entityTranslucentCullItemTarget uses OutputTarget.ITEM_ENTITY_TARGET before particles/combine"
        );
        assert!(
            source[item_helper..main_helper].contains("entity_model_item_entity_translucent_mesh")
                && source[item_helper..main_helper]
                    .contains("entity_dynamic_player_skin_item_entity_translucent_mesh")
                && source[item_helper..main_helper]
                    .contains("entity_dynamic_player_texture_item_entity_translucent_mesh"),
            "static and dynamic item-target translucent entity meshes draw through the item_entity target helper"
        );
        assert!(
            !source[main_helper..tests_mod].contains("entity_model_item_entity_translucent_mesh"),
            "main-target translucent feature helper must not draw ITEM_ENTITY_TARGET render types"
        );
    }

    #[test]
    fn particle_target_copies_main_depth_and_clears_before_combine() {
        let source = include_str!("render.rs");
        let entity_translucent_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let target = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let copy_depth = depth_copy_to(source, "texture: &self.particle_target.depth._texture");
        let particle_pipeline = source[target..]
            .find("pass.set_pipeline(&self.particle_pipeline)")
            .map(|index| target + index)
            .expect("particle pipeline is drawn into the target");
        let particle_atlas = source[particle_pipeline..]
            .find("pass.set_bind_group(0, &atlas.bind_group, &[])")
            .map(|index| particle_pipeline + index)
            .expect("particle atlas bind group is bound before draw");
        let particle_lightmap = source[particle_atlas..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| particle_atlas + index)
            .expect("particle lightmap bind group is bound before draw");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        assert!(
            copy_depth < entity_translucent_features
                && entity_translucent_features < target
                && target < particle_pipeline
                && particle_lightmap < combine,
            "particle target copies main depth, clears transparent, draws particles, then transparency combine consumes it"
        );
        assert!(
            particle_pipeline < particle_atlas && particle_atlas < particle_lightmap,
            "particles bind the renderer-owned LightTexture before drawing"
        );
        assert!(
            source[copy_depth..entity_translucent_features]
                .contains("texture: &self.depth._texture")
                && source[copy_depth..entity_translucent_features]
                    .contains("texture: &self.particle_target.depth._texture"),
            "particle target depth is copied from the renderer-owned main depth texture"
        );
        assert!(
            source[target..particle_pipeline]
                .contains("load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)"),
            "particle target color is cleared every frame so missing particle draws do not reuse stale color"
        );
        assert!(
            source[target..combine].contains("view: particle_view")
                && source[target..combine].contains("view: &self.particle_target.depth.view"),
            "particle geometry renders into the particles color/depth target"
        );
    }

    #[test]
    fn weather_target_copies_main_depth_and_clears_before_combine() {
        let source = include_str!("render.rs");
        let particle_target = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let target = source
            .find("label: Some(WEATHER_TARGET_PASS_LABEL)")
            .expect("weather target pass label is used");
        let copy_depth = source[particle_target..target]
            .rfind("encoder.copy_texture_to_texture")
            .map(|index| particle_target + index)
            .expect("main depth is copied into weather target depth");
        let weather_pipeline = source[target..]
            .find("pass.set_pipeline(&self.weather_pipeline)")
            .map(|index| target + index)
            .expect("weather pipeline is drawn into the target");
        let weather_lightmap = source[weather_pipeline..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| weather_pipeline + index)
            .expect("weather binds the renderer-owned LightTexture");
        let rain_bind = source[weather_lightmap..]
            .find("pass.set_bind_group(0, &texture.bind_group, &[])")
            .map(|index| weather_lightmap + index)
            .expect("rain texture bind group is bound before rain draw");
        let rain_draw = source[rain_bind..]
            .find("pass.draw_indexed(mesh.rain_indices.clone(), 0, 0..1)")
            .map(|index| rain_bind + index)
            .expect("rain index range is drawn first");
        let snow_draw = source[rain_draw..]
            .find("pass.draw_indexed(mesh.snow_indices.clone(), 0, 0..1)")
            .map(|index| rain_draw + index)
            .expect("snow index range is drawn after rain");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        assert!(
            copy_depth < target
                && target < weather_pipeline
                && weather_lightmap < rain_bind
                && rain_bind < rain_draw
                && rain_draw < snow_draw
                && snow_draw < combine,
            "weather target copies main depth, clears transparent, draws rain then snow, and then transparency combine consumes it"
        );
        assert!(
            source[copy_depth..target].contains("texture: &self.depth._texture")
                && source[copy_depth..target]
                    .contains("texture: &self.weather_target.depth._texture"),
            "weather target depth is copied from the renderer-owned main depth texture"
        );
        assert!(
            source[target..combine]
                .contains("load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)"),
            "weather target color is cleared every frame so a future sparse weather draw cannot reuse stale color"
        );
        assert!(
            source[target..combine].contains("view: weather_view")
                && source[target..combine].contains("view: &self.weather_target.depth.view"),
            "weather pass owns the weather color/depth target"
        );
        assert!(
            source[target..combine]
                .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])"),
            "weather geometry samples the renderer-owned dynamic LightTexture"
        );
    }

    #[test]
    fn lightning_draws_into_weather_target_before_rain_snow_and_combine() {
        let source = include_str!("render.rs");
        let target = source
            .find("label: Some(WEATHER_TARGET_PASS_LABEL)")
            .expect("weather target pass label is used");
        let lightning = source[target..]
            .find("pass.set_pipeline(&self.lightning_pipeline)")
            .map(|index| target + index)
            .expect("weather target pass draws lightning pipeline");
        let weather = source[target..]
            .find("pass.set_pipeline(&self.weather_pipeline)")
            .map(|index| target + index)
            .expect("weather target pass draws rain/snow pipeline");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        assert!(
            target < lightning && lightning < weather && weather < combine,
            "vanilla RenderTypes.lightning writes WEATHER_TARGET before WeatherEffectRenderer rain/snow and transparency combine"
        );
        assert!(
            source[lightning..weather].contains("pass.set_bind_group(0, &self.terrain_bind_group"),
            "lightning uses the renderer camera bind group and no weather texture"
        );
    }

    #[test]
    fn transparency_combine_writes_surface_before_hud_and_screenshot_readback() {
        let source = include_str!("render.rs");
        let main_view = source
            .find("let main_view = &self.main_target.view")
            .expect("renderer-owned main target view is selected");
        let terrain_pass = source
            .find("label: Some(\"bbb-native-terrain-opaque-group-pass\")")
            .expect("main terrain pass label is used");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");
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
            main_view < terrain_pass && terrain_pass < combine,
            "content passes draw to the renderer-owned main target before transparency combine"
        );
        assert!(
            !source[..combine].contains("view: &surface_view"),
            "surface view is not a render target until the transparency combine pass"
        );
        assert!(
            source[terrain_pass..combine].contains("view: main_view"),
            "main content passes use the renderer-owned main target"
        );
        assert!(
            source[combine..hud_pass].contains("view: &surface_view"),
            "transparency combine writes the swapchain surface before HUD rendering"
        );
        assert!(
            source[combine..hud_pass].contains(
                "pass.set_bind_group(0, &self.transparency_combine_bind_group.bind_group, &[])"
            ),
            "transparency combine samples the renderer-owned target bundle"
        );
        assert!(
            combine < hud_pass && hud_pass < hud_item_pass && hud_item_pass < screenshot_copy,
            "HUD and GUI item passes draw on the surface after transparency combine"
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
    fn entity_outline_target_composites_after_complete_main_pass_before_clouds() {
        // Vanilla LevelRenderer adds the entity_outline post chain immediately after addMainPass
        // and before clouds, weather, and transparency. Keep bbb's current target write plus
        // post-chain after the target-backed main-pass work until the renderer splits target
        // writes into the main pass resource scope.
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
        let particle = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let clouds = source
            .find("label: Some(CLOUDS_PASS_LABEL)")
            .expect("cloud pass label is used");
        let translucent = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");
        let weather = source
            .find("label: Some(WEATHER_TARGET_PASS_LABEL)")
            .expect("weather target pass label is used");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");
        assert!(
            target < sobel
                && sobel < blur_horizontal
                && blur_horizontal < blur_vertical
                && blur_vertical < blit
                && blit < composite,
            "outline target and post-chain passes follow vanilla entity_outline.json order"
        );
        assert!(
            translucent < particle
                && particle < target
                && composite < clouds
                && clouds < weather
                && weather < combine,
            "outline post-chain runs after the complete target-backed main pass and before vanilla cloud/weather/transparency passes"
        );
    }
}
