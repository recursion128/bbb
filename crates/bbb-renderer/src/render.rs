use std::path::Path;

use anyhow::Result;
use wgpu::util::DeviceExt;

use crate::frame_buffers::FrameDataBuffer;
use crate::renderer::FrameTarget;
use crate::{
    clouds::CloudShape,
    entity_models::{
        upload_elder_guardian_particle_textured_mesh,
        upload_experience_orb_pickup_particle_textured_mesh,
        upload_projectile_pickup_particle_textured_mesh, EntityModelLayerRenderType,
        EntityModelMeshGpu, EntityModelPositionColorDrawRange, EntityModelScrollDrawRange,
        EntityModelTexturedDrawAtlas, EntityModelTexturedDrawRange, EntityModelTexturedMeshGpu,
        EntityModelTranslucentDrawRange,
    },
    lightmap::write_lightmap_uniform,
    particles::{
        ParticleAtlasDrawRange, ParticlePipelineVertexBatch, ParticleTextureAtlasKind,
        ParticleVertexBatches,
    },
    weather::{build_lightning_mesh, build_weather_mesh},
    world_border::build_world_border_mesh,
    Renderer,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerrainOpaqueGroupLayer {
    Solid,
    Cutout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HudActivePipeline {
    Sprite,
    ItemGlint,
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
const ITEM_ENTITY_LINE_TARGET_PASS_LABEL: &str = "bbb-native-item-entity-line-target-pass";
const OPAQUE_PARTICLE_MAIN_PASS_LABEL: &str = "bbb-native-opaque-particle-main-pass";
const PARTICLE_TARGET_PASS_LABEL: &str = "bbb-native-particle-target-pass";
const WEATHER_TARGET_PASS_LABEL: &str = "bbb-native-weather-target-pass";
const FIRST_PERSON_ITEM_PASS_LABEL: &str = "bbb-native-first-person-item-pass";
const LIGHTMAP_PASS_LABEL: &str = "bbb-native-lightmap-pass";
const TRANSPARENCY_COMBINE_PASS_LABEL: &str = "bbb-native-transparency-combine-pass";
const TRANSPARENCY_BLIT_PASS_LABEL: &str = "bbb-native-transparency-blit-pass";

struct ItemModelFrameBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// Per-frame draw-call and pipeline-switch tallies, accumulated across the
/// frame steps and folded into `RendererCounters` by `finish_frame`.
#[derive(Default)]
struct FrameDrawStats {
    opaque_draw_calls: u64,
    cutout_draw_calls: u64,
    translucent_draw_calls: u64,
    block_destroy_overlay_draw_calls: u64,
    sky_draw_calls: u64,
    entity_model_draw_calls: u64,
    outline_composite_draw_calls: u64,
    transparency_combine_draw_calls: u64,
    transparency_blit_draw_calls: u64,
    particle_draw_calls: u64,
    weather_draw_calls: u64,
    item_entity_draw_calls: u64,
    item_model_draw_calls: u64,
    selection_draw_calls: u64,
    entity_scene_draw_calls: u64,
    entity_target_draw_calls: u64,
    hud_draw_calls: u64,
    lightmap_draw_calls: u64,
    pipeline_switches: u64,
}

impl Renderer {
    pub fn render(&mut self, screenshot: Option<&Path>) -> Result<()> {
        let Some(frame) = self.surface.acquire_frame(&self.device, &self.config)? else {
            return Ok(());
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-native-clear"),
            });
        let mut stats = FrameDrawStats::default();
        write_lightmap_uniform(
            &self.queue,
            &self.lightmap.uniform_buffer,
            self.lightmap_environment,
        );
        self.lightmap_pass(&mut encoder, &mut stats);
        self.main_world_pass(&mut encoder, &mut stats);
        let particle_vertex_batches = self.opaque_particle_main_pass(&mut encoder, &mut stats);
        self.copy_main_depth_to_feature_targets(&mut encoder);
        self.entity_translucent_feature_pass(&mut encoder, &mut stats);
        self.item_entity_target_pass(&mut encoder, &mut stats);
        self.block_destroy_overlay_pass(&mut encoder, &mut stats);
        self.entity_outline_target_pass(&mut encoder, &mut stats);
        self.translucent_target_pass(&mut encoder, &mut stats);
        self.item_entity_line_target_pass(&mut encoder, &mut stats);
        self.particle_target_pass(&mut encoder, &mut stats, particle_vertex_batches);
        self.entity_outline_post_chain(&mut encoder, &mut stats);
        self.clouds_pass(&mut encoder, &mut stats);
        self.weather_target_pass(&mut encoder, &mut stats);
        self.transparency_combine_pass(&mut encoder, &mut stats);
        self.transparency_blit_pass(&frame, &mut encoder, &mut stats);
        self.first_person_item_pass(&frame, &mut encoder, &mut stats);
        self.entity_preview_pip_passes(&mut encoder, &mut stats);
        self.hud_passes(&frame, &mut encoder, &mut stats);
        self.finish_frame(encoder, frame, screenshot, stats)
    }

    fn lightmap_pass(&self, encoder: &mut wgpu::CommandEncoder, stats: &mut FrameDrawStats) {
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
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.lightmap.bind_group, &[]);
            pass.draw(0..3, 0..1);
            stats.lightmap_draw_calls += 1;
        }
    }

    fn main_world_pass(&self, encoder: &mut wgpu::CommandEncoder, stats: &mut FrameDrawStats) {
        let main_view = &self.main_target.view;
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
                    stats.pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &end_sky_texture.bind_group, &[]);
                    pass.set_bind_group(2, &self.end_sky_mesh.dynamic.bind_group, &[]);
                    pass.set_vertex_buffer(0, self.end_sky_mesh.vertex_buffer.slice(..));
                    pass.draw(0..self.end_sky_mesh.vertex_count, 0..1);
                    stats.sky_draw_calls += 1;
                }
            } else if let Some(sky_disc) = &self.sky_disc {
                if sky_disc.disc_vertex_count > 0 {
                    pass.set_pipeline(&self.sky_pipeline);
                    stats.pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &sky_disc.dynamic.bind_group, &[]);
                    let vertex_buffer = sky_disc
                        .disc_vertex_buffer
                        .as_ref()
                        .expect("sky disc vertex buffer exists when count is non-zero");
                    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    pass.draw(0..sky_disc.disc_vertex_count, 0..1);
                    stats.sky_draw_calls += 1;
                }

                if sky_disc.sunrise_vertex_count > 0 {
                    pass.set_pipeline(&self.sunrise_sunset_pipeline);
                    stats.pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    let vertex_buffer = sky_disc
                        .sunrise_vertex_buffer
                        .as_ref()
                        .expect("sunrise/sunset vertex buffer exists when count is non-zero");
                    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    pass.draw(0..sky_disc.sunrise_vertex_count, 0..1);
                    stats.sky_draw_calls += 1;
                }

                if let (Some(celestial_atlas), Some(celestials)) =
                    (&self.celestial_atlas, &self.sky_celestials)
                {
                    pass.set_pipeline(&self.celestial_pipeline);
                    stats.pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &celestial_atlas.bind_group, &[]);
                    pass.set_bind_group(2, &celestials.sun.dynamic.bind_group, &[]);
                    pass.set_vertex_buffer(0, celestials.sun.vertex_buffer.slice(..));
                    pass.draw(0..celestials.sun.vertex_count, 0..1);
                    stats.sky_draw_calls += 1;
                    pass.set_bind_group(2, &celestials.moon.dynamic.bind_group, &[]);
                    pass.set_vertex_buffer(0, celestials.moon.vertex_buffer.slice(..));
                    pass.draw(0..celestials.moon.vertex_count, 0..1);
                    stats.sky_draw_calls += 1;
                }

                if let Some(stars) = &self.sky_stars {
                    pass.set_pipeline(&self.star_pipeline);
                    stats.pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &stars.dynamic.bind_group, &[]);
                    pass.set_vertex_buffer(0, stars.vertex_buffer.slice(..));
                    pass.draw(0..stars.vertex_count, 0..1);
                    stats.sky_draw_calls += 1;
                }
            }

            // Vanilla 26.1 renders ChunkSectionLayerGroup.OPAQUE as SOLID then CUTOUT
            // before feature submissions; keep both terrain layers ahead of entity draws.
            for terrain_layer in TERRAIN_OPAQUE_GROUP_LAYERS {
                match terrain_layer {
                    TerrainOpaqueGroupLayer::Solid => {
                        if !self.terrain_opaque.is_empty() {
                            pass.set_pipeline(&self.terrain_pipeline);
                            stats.pipeline_switches += 1;
                            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                            for mesh in &self.terrain_opaque {
                                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                pass.set_index_buffer(
                                    mesh.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                                stats.opaque_draw_calls += 1;
                            }
                        }
                    }
                    TerrainOpaqueGroupLayer::Cutout => {
                        if !self.terrain_cutout.is_empty() {
                            pass.set_pipeline(&self.terrain_pipeline);
                            stats.pipeline_switches += 1;
                            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                            for mesh in &self.terrain_cutout {
                                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                pass.set_index_buffer(
                                    mesh.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                                stats.cutout_draw_calls += 1;
                            }
                        }
                    }
                }
            }
            if let Some(mesh) = &self.entity_model_mesh {
                pass.set_pipeline(&self.entity_model_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_textured_cull_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_cull_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_textured_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            // Vanilla `RenderTypes.entityCutoutDissolve` (the dying ender dragon body): opaque cutout
            // family, drawn in the main pass through the dedicated DISSOLVE-mask pipeline.
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_dissolve_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_dissolve_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_cutout_z_offset_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_cutout_z_offset_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_armor_cutout_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_armor_cutout_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_skin_cutout_cull_mesh,
                &self.entity_dynamic_player_skin_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_cull_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_skin_cutout_mesh,
                &self.entity_dynamic_player_skin_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_skin_cutout_z_offset_mesh,
                &self.entity_dynamic_player_skin_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_cutout_z_offset_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_texture_cutout_cull_mesh,
                &self.entity_dynamic_player_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_cull_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_texture_cutout_mesh,
                &self.entity_dynamic_player_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_textured_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_texture_cutout_z_offset_mesh,
                &self.entity_dynamic_player_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_cutout_z_offset_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_dynamic_player_texture_armor_cutout_mesh,
                &self.entity_dynamic_player_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_armor_cutout_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let Some(mesh) = &self.entity_model_water_mask_mesh {
                pass.set_pipeline(&self.entity_model_water_mask_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_armor_entity_glint_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_armor_entity_glint_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_entity_glint_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_entity_glint_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
        }

        // Vanilla solid item features are part of FeatureRenderDispatcher.renderSolidFeatures()
        // inside the main pass: after model/model-part features and before target depth copies,
        // translucent terrain, entity-outline post-chain work, and clouds. GUI item icons use
        // the later HUD item pass instead.
        let (block_item_vertices, block_item_indices) = self.collect_block_item_model_geometry();
        if !block_item_indices.is_empty() {
            self.draw_item_model_geometry(
                &mut *encoder,
                main_view,
                &block_item_vertices,
                &block_item_indices,
                &self.terrain_bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        let (block_item_z_offset_forward_vertices, block_item_z_offset_forward_indices) =
            self.collect_block_item_model_z_offset_forward_geometry();
        if !block_item_z_offset_forward_indices.is_empty() {
            self.draw_item_model_geometry_with_pipeline(
                &mut *encoder,
                main_view,
                &block_item_z_offset_forward_vertices,
                &block_item_z_offset_forward_indices,
                &self.terrain_bind_group,
                &self.item_model_z_offset_forward_pipeline,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        let (map_vertices, map_indices) = self.collect_item_frame_map_geometry();
        if !map_indices.is_empty() {
            if let Some(atlas) = &self.item_frame_map_atlas {
                self.draw_item_model_geometry(
                    &mut *encoder,
                    main_view,
                    &map_vertices,
                    &map_indices,
                    &atlas.bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
        }
        let (map_decoration_vertices, map_decoration_indices) =
            self.collect_item_frame_map_decoration_geometry();
        if !map_decoration_indices.is_empty() {
            if let Some(atlas) = &self.item_frame_map_decoration_atlas {
                self.draw_item_model_geometry(
                    &mut *encoder,
                    main_view,
                    &map_decoration_vertices,
                    &map_decoration_indices,
                    &atlas.bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
        }
        let (flat_item_vertices, flat_item_indices) = self.collect_flat_item_model_geometry();
        if !flat_item_indices.is_empty() {
            if let Some(atlas) = &self.item_entity_atlas {
                self.draw_item_model_geometry(
                    &mut *encoder,
                    main_view,
                    &flat_item_vertices,
                    &flat_item_indices,
                    &atlas.bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
        }
        let (item_model_glint_vertices, item_model_glint_indices) =
            self.collect_item_model_glint_geometry();
        if !item_model_glint_indices.is_empty() {
            if let Some(glint) = &self.item_glint_texture {
                self.draw_item_model_glint_geometry(
                    &mut *encoder,
                    main_view,
                    &item_model_glint_vertices,
                    &item_model_glint_indices,
                    &glint.main_bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
        }
    }

    /// Vanilla `ParticleFeatureRenderer.render` (26.1 lines 46-57) picks the render target with
    /// `useParticleTarget = particleTarget != null && translucent`: opaque particles
    /// (`translucent == false`) draw into the **main** color+depth attachment during
    /// `renderSolidFeatures`, only translucent particles use the dedicated particles target.
    /// In `LevelRenderer.addMainPass` (26.1) that solid pass runs before the three
    /// `copyDepthFrom` calls (lines 680-689), so opaque-particle depth propagates into every
    /// feature target. Mirroring that, opaque particles draw here into main just before
    /// `copy_main_depth_to_feature_targets`; translucent particles stay in
    /// `particle_target_pass`.
    ///
    /// Both particle pipelines share vanilla `DepthStencilState.DEFAULT`
    /// (`CompareOp.LESS_THAN_OR_EQUAL`, depth write on), so drawing the opaque pipeline against
    /// the main depth attachment writes depth exactly like vanilla `OPAQUE_PARTICLE`.
    ///
    /// The full particle vertex batches (opaque + translucent) are collected once here and
    /// returned so `particle_target_pass` can upload and draw the translucent half without
    /// rebuilding the CPU billboard geometry.
    fn opaque_particle_main_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) -> ParticleVertexBatches {
        let main_view = &self.main_target.view;
        let particle_vertex_batches = self.collect_particle_vertex_batches();
        let has_opaque_particles = self.frame_opaque_particle_vertices.upload(
            &self.device,
            &self.queue,
            bytemuck::cast_slice(&particle_vertex_batches.opaque.vertices),
        );
        if let Some(vertex_buffer) =
            has_opaque_particles.then(|| self.frame_opaque_particle_vertices.buffer())
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(OPAQUE_PARTICLE_MAIN_PASS_LABEL),
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
            pass.set_pipeline(&self.opaque_particle_pipeline);
            stats.pipeline_switches += 1;
            self.draw_particle_vertex_batch(
                &mut pass,
                vertex_buffer.expect("uploaded"),
                &particle_vertex_batches.opaque,
                stats,
            );
        }
        particle_vertex_batches
    }

    fn copy_main_depth_to_feature_targets(&self, encoder: &mut wgpu::CommandEncoder) {
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
    }

    fn entity_translucent_feature_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let main_view = &self.main_target.view;
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
                &mut stats.pipeline_switches,
                &mut stats.entity_model_draw_calls,
            );
        }

        // Vanilla MapRenderer submits decoration name labels through
        // submitNodeCollector.order(1).submitText(...), so TextFeatureRenderer draws them during
        // renderTranslucentFeatures after order-0 model/custom geometry and before crumbling /
        // translucent terrain. The current renderer only has item-frame map labels for this path.
        let (map_text_vertices, map_text_indices) = self.collect_item_frame_map_text_geometry();
        if !map_text_indices.is_empty() {
            if let Some(atlas) = &self.item_frame_map_text_font_atlas {
                self.draw_item_model_geometry(
                    &mut *encoder,
                    main_view,
                    &map_text_vertices,
                    &map_text_indices,
                    &atlas.bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
        }

        // Vanilla AbstractSignRenderer.submitSignText also goes through
        // submitText (drawn by TextFeatureRenderer in the same translucent
        // feature phase), so sign face text draws here with the same
        // font/default atlas as the map labels.
        let (sign_text_vertices, sign_text_indices) = self.collect_sign_text_geometry();
        if !sign_text_indices.is_empty() {
            if let Some(atlas) = &self.item_frame_map_text_font_atlas {
                self.draw_item_model_geometry(
                    &mut *encoder,
                    main_view,
                    &sign_text_vertices,
                    &sign_text_indices,
                    &atlas.bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
        }
    }

    fn item_entity_target_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let item_entity_view = &self.item_entity_target.view;
        let item_entity_vertices = self.collect_item_entity_vertices();
        let has_item_entity_vertices = self.item_entity_atlas.is_some()
            && self.frame_item_entity_vertices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&item_entity_vertices),
            );
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
        let (item_model_glint_translucent_vertices, item_model_glint_translucent_indices) =
            self.collect_item_model_glint_translucent_geometry();
        let item_model_glint_translucent_buffers = self.create_item_model_frame_buffers(
            &item_model_glint_translucent_vertices,
            &item_model_glint_translucent_indices,
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
                &mut stats.pipeline_switches,
                &mut stats.entity_model_draw_calls,
            );
            if let Some(buffers) = &block_item_translucent_buffers {
                self.draw_item_model_frame_buffers(
                    &mut pass,
                    &self.item_model_translucent_pipeline,
                    buffers,
                    &self.terrain_bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
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
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let (Some(glint), Some(buffers)) = (
                &self.item_glint_texture,
                &item_model_glint_translucent_buffers,
            ) {
                self.draw_item_model_glint_frame_buffers(
                    &mut pass,
                    buffers,
                    &glint.main_bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let (Some(atlas), true) = (&self.item_entity_atlas, has_item_entity_vertices) {
                let vertex_buffer = self.frame_item_entity_vertices.buffer().expect("uploaded");
                pass.set_pipeline(&self.item_entity_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..item_entity_vertices.len() as u32, 0..1);
                stats.item_entity_draw_calls += 1;
            }
        }
    }

    fn block_destroy_overlay_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let main_view = &self.main_target.view;
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
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, overlays.vertex_buffer.slice(..));
            pass.set_index_buffer(overlays.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..overlays.index_count, 0, 0..1);
            stats.block_destroy_overlay_draw_calls += 1;
        }
    }

    fn entity_outline_target_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let has_entity_outline_meshes = self.entity_model_texture_atlas.is_some()
            && (self.entity_model_outline_mesh.is_some()
                || self.entity_model_outline_cull_mesh.is_some());
        if has_entity_outline_meshes {
            let atlas = self
                .entity_model_texture_atlas
                .as_ref()
                .expect("outline meshes require the static entity atlas");
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
                stats.pipeline_switches += 1;
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let Some(mesh) = &self.entity_model_outline_cull_mesh {
                pass.set_pipeline(&self.entity_model_outline_cull_pipeline);
                stats.pipeline_switches += 1;
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
        }
    }

    fn translucent_target_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let translucent_view = &self.translucent_target.view;
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
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                // Draw sections far→near via `terrain_translucent_order`, the
                // camera-sorted permutation of `terrain_translucent` that mirrors
                // vanilla's reversed TRANSLUCENT draw list
                // (ChunkSectionsToRender.java:55-56, MC 26.1). The order is kept
                // in lock-step with the mesh set, but fall back to storage order
                // if it is ever stale so no section is silently dropped.
                let ordered =
                    self.terrain_translucent_order.len() == self.terrain_translucent.len();
                for draw_index in 0..self.terrain_translucent.len() {
                    let mesh = if ordered {
                        &self.terrain_translucent[self.terrain_translucent_order[draw_index]]
                    } else {
                        &self.terrain_translucent[draw_index]
                    };
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                    stats.translucent_draw_calls += 1;
                }
            }
        }
    }

    fn item_entity_line_target_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let item_entity_view = &self.item_entity_target.view;
        if self.selection_outline.is_some()
            || self.entity_scene_outline.is_some()
            || self.entity_target_outline.is_some()
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(ITEM_ENTITY_LINE_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: item_entity_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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
            pass.set_pipeline(&self.selection_pipeline);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            if let Some(outline) = &self.selection_outline {
                pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
                pass.draw(0..outline.vertex_count, 0..1);
                stats.selection_draw_calls += 1;
            }
            if let Some(outline) = &self.entity_scene_outline {
                pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
                pass.draw(0..outline.vertex_count, 0..1);
                stats.entity_scene_draw_calls += 1;
            }
            if let Some(outline) = &self.entity_target_outline {
                pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
                pass.draw(0..outline.vertex_count, 0..1);
                stats.entity_target_draw_calls += 1;
            }
        }
    }

    fn particle_target_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
        particle_vertex_batches: ParticleVertexBatches,
    ) {
        let particle_view = &self.particle_target.view;
        // Opaque particles already drew into main color+depth in `opaque_particle_main_pass`;
        // only the translucent half renders into the dedicated particles target here, matching
        // vanilla `ParticleFeatureRenderer.renderTranslucent`.
        let has_translucent_particles = self.frame_translucent_particle_vertices.upload(
            &self.device,
            &self.queue,
            bytemuck::cast_slice(&particle_vertex_batches.translucent.vertices),
        );
        let (item_pickup_block_vertices, item_pickup_block_indices) =
            self.collect_item_pickup_block_item_model_geometry();
        let item_pickup_block_buffers = self.create_item_model_frame_buffers(
            &item_pickup_block_vertices,
            &item_pickup_block_indices,
        );
        let (item_pickup_flat_vertices, item_pickup_flat_indices) =
            self.collect_item_pickup_flat_item_model_geometry();
        let item_pickup_flat_buffers = self
            .create_item_model_frame_buffers(&item_pickup_flat_vertices, &item_pickup_flat_indices);
        let (item_pickup_glint_vertices, item_pickup_glint_indices) =
            self.collect_item_pickup_item_model_glint_geometry();
        let item_pickup_glint_buffers = self.create_item_model_frame_buffers(
            &item_pickup_glint_vertices,
            &item_pickup_glint_indices,
        );
        let (item_pickup_block_translucent_vertices, item_pickup_block_translucent_indices) =
            self.collect_item_pickup_block_item_model_translucent_geometry();
        let item_pickup_block_translucent_buffers = self.create_item_model_frame_buffers(
            &item_pickup_block_translucent_vertices,
            &item_pickup_block_translucent_indices,
        );
        let (item_pickup_flat_translucent_vertices, item_pickup_flat_translucent_indices) =
            self.collect_item_pickup_flat_item_model_translucent_geometry();
        let item_pickup_flat_translucent_buffers = self.create_item_model_frame_buffers(
            &item_pickup_flat_translucent_vertices,
            &item_pickup_flat_translucent_indices,
        );
        let (item_pickup_glint_translucent_vertices, item_pickup_glint_translucent_indices) =
            self.collect_item_pickup_item_model_glint_translucent_geometry();
        let item_pickup_glint_translucent_buffers = self.create_item_model_frame_buffers(
            &item_pickup_glint_translucent_vertices,
            &item_pickup_glint_translucent_indices,
        );
        let experience_orb_pickup_particles =
            self.collect_experience_orb_pickup_particle_render_instances();
        let experience_orb_pickup_particle_index_count =
            if let Some(atlas) = &self.entity_model_texture_atlas {
                upload_experience_orb_pickup_particle_textured_mesh(
                    &self.device,
                    &self.queue,
                    &mut self.frame_experience_orb_pickup_particle_vertices,
                    &mut self.frame_experience_orb_pickup_particle_indices,
                    &experience_orb_pickup_particles,
                    &atlas.layout,
                )
            } else {
                None
            };
        let projectile_pickup_particles =
            self.collect_projectile_pickup_particle_render_instances();
        let projectile_pickup_particle_index_count =
            if let Some(atlas) = &self.entity_model_texture_atlas {
                upload_projectile_pickup_particle_textured_mesh(
                    &self.device,
                    &self.queue,
                    &mut self.frame_projectile_pickup_particle_vertices,
                    &mut self.frame_projectile_pickup_particle_indices,
                    &projectile_pickup_particles,
                    &atlas.layout,
                )
            } else {
                None
            };
        let elder_guardian_particles = self.collect_elder_guardian_particle_render_instances();
        let elder_guardian_particle_index_count =
            if let Some(atlas) = &self.entity_model_texture_atlas {
                upload_elder_guardian_particle_textured_mesh(
                    &self.device,
                    &self.queue,
                    &mut self.frame_elder_guardian_particle_vertices,
                    &mut self.frame_elder_guardian_particle_indices,
                    &elder_guardian_particles,
                    &atlas.layout,
                )
            } else {
                None
            };
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
            if let Some(vertex_buffer) =
                has_translucent_particles.then(|| self.frame_translucent_particle_vertices.buffer())
            {
                pass.set_pipeline(&self.translucent_particle_pipeline);
                stats.pipeline_switches += 1;
                self.draw_particle_vertex_batch(
                    &mut pass,
                    vertex_buffer.expect("uploaded"),
                    &particle_vertex_batches.translucent,
                    stats,
                );
            }
            if let Some(buffers) = &item_pickup_block_buffers {
                self.draw_item_model_frame_buffers(
                    &mut pass,
                    &self.item_model_pipeline,
                    buffers,
                    &self.terrain_bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let (Some(atlas), Some(buffers)) =
                (&self.item_entity_atlas, &item_pickup_flat_buffers)
            {
                self.draw_item_model_frame_buffers(
                    &mut pass,
                    &self.item_model_pipeline,
                    buffers,
                    &atlas.bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let (Some(glint), Some(buffers)) =
                (&self.item_glint_texture, &item_pickup_glint_buffers)
            {
                self.draw_item_model_glint_frame_buffers(
                    &mut pass,
                    buffers,
                    &glint.main_bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let Some(buffers) = &item_pickup_block_translucent_buffers {
                self.draw_item_model_frame_buffers(
                    &mut pass,
                    &self.item_model_translucent_pipeline,
                    buffers,
                    &self.terrain_bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let (Some(atlas), Some(buffers)) = (
                &self.item_entity_atlas,
                &item_pickup_flat_translucent_buffers,
            ) {
                self.draw_item_model_frame_buffers(
                    &mut pass,
                    &self.item_model_translucent_pipeline,
                    buffers,
                    &atlas.bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let (Some(glint), Some(buffers)) = (
                &self.item_glint_texture,
                &item_pickup_glint_translucent_buffers,
            ) {
                self.draw_item_model_glint_frame_buffers(
                    &mut pass,
                    buffers,
                    &glint.main_bind_group,
                );
                stats.pipeline_switches += 1;
                stats.item_model_draw_calls += 1;
            }
            if let (Some(index_count), Some(atlas)) = (
                experience_orb_pickup_particle_index_count,
                self.entity_model_texture_atlas.as_ref(),
            ) {
                pass.set_pipeline(&self.entity_model_translucent_cull_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(
                    0,
                    self.frame_experience_orb_pickup_particle_vertices
                        .buffer()
                        .expect("experience orb pickup particle vertices uploaded")
                        .slice(..),
                );
                pass.set_index_buffer(
                    self.frame_experience_orb_pickup_particle_indices
                        .buffer()
                        .expect("experience orb pickup particle indices uploaded")
                        .slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            // Item-pickup carried arrow/trident models (vanilla
            // `ItemPickupParticleGroup.State.submit` -> `ArrowRenderer` /
            // `ThrownTridentRenderer`): drawn inside the ITEM_PICKUP group after
            // the item-cluster and orb-icon draws, through the same
            // translucent-cull entity pipeline as the orb billboard.
            if let (Some(index_count), Some(atlas)) = (
                projectile_pickup_particle_index_count,
                self.entity_model_texture_atlas.as_ref(),
            ) {
                pass.set_pipeline(&self.entity_model_translucent_cull_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(
                    0,
                    self.frame_projectile_pickup_particle_vertices
                        .buffer()
                        .expect("projectile pickup particle vertices uploaded")
                        .slice(..),
                );
                pass.set_index_buffer(
                    self.frame_projectile_pickup_particle_indices
                        .buffer()
                        .expect("projectile pickup particle indices uploaded")
                        .slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..index_count, 0, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if let (Some(index_count), Some(atlas)) = (
                elder_guardian_particle_index_count,
                self.entity_model_texture_atlas.as_ref(),
            ) {
                pass.set_pipeline(&self.entity_model_translucent_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(
                    0,
                    self.frame_elder_guardian_particle_vertices
                        .buffer()
                        .expect("elder guardian particle vertices uploaded")
                        .slice(..),
                );
                pass.set_index_buffer(
                    self.frame_elder_guardian_particle_indices
                        .buffer()
                        .expect("elder guardian particle indices uploaded")
                        .slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..index_count, 0, 0..1);
                stats.particle_draw_calls += 1;
            }
        }
    }

    fn entity_outline_post_chain(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let main_view = &self.main_target.view;
        let has_entity_outline_meshes = self.entity_model_texture_atlas.is_some()
            && (self.entity_model_outline_mesh.is_some()
                || self.entity_model_outline_cull_mesh.is_some());
        if has_entity_outline_meshes {
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
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.bind_group, &[]);
                pass.draw(0..3, 0..1);
                stats.outline_composite_draw_calls += 1;
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
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.swap_linear_bind_group, &[]);
                pass.draw(0..3, 0..1);
                stats.outline_composite_draw_calls += 1;
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
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.linear_bind_group, &[]);
                pass.draw(0..3, 0..1);
                stats.outline_composite_draw_calls += 1;
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
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.entity_outline_target.swap_bind_group, &[]);
                pass.draw(0..3, 0..1);
                stats.outline_composite_draw_calls += 1;
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
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.entity_outline_target.bind_group, &[]);
            pass.draw(0..3, 0..1);
            stats.outline_composite_draw_calls += 1;
        }
    }

    fn clouds_pass(&self, encoder: &mut wgpu::CommandEncoder, stats: &mut FrameDrawStats) {
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
                    let cloud_pipeline = match self.cloud_shape {
                        CloudShape::Flat => &self.cloud_flat_pipeline,
                        CloudShape::Fancy => &self.cloud_fancy_pipeline,
                    };
                    pass.set_pipeline(cloud_pipeline);
                    stats.pipeline_switches += 1;
                    pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                    pass.set_bind_group(1, &self.cloud_bind_group, &[]);
                    pass.set_vertex_buffer(0, clouds.vertex_buffer.slice(..));
                    pass.draw(0..clouds.vertex_count, 0..1);
                    stats.sky_draw_calls += 1;
                }
            }
        }
    }

    fn weather_target_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let weather_view = &self.weather_target.view;
        let weather_mesh = build_weather_mesh(&self.weather_render_state);
        let has_weather_buffers = weather_mesh
            .as_ref()
            .filter(|mesh| {
                (!mesh.rain_indices.is_empty() && self.weather_rain_texture.is_some())
                    || (!mesh.snow_indices.is_empty() && self.weather_snow_texture.is_some())
            })
            .is_some_and(|mesh| {
                self.frame_weather_vertices.upload(
                    &self.device,
                    &self.queue,
                    bytemuck::cast_slice(&mesh.vertices),
                ) && self.frame_weather_indices.upload(
                    &self.device,
                    &self.queue,
                    bytemuck::cast_slice(&mesh.indices),
                )
            });
        let lightning_mesh = build_lightning_mesh(&self.weather_render_state);
        let has_lightning_buffers = lightning_mesh.as_ref().is_some_and(|mesh| {
            self.frame_lightning_vertices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&mesh.vertices),
            ) && self.frame_lightning_indices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&mesh.indices),
            )
        });
        let world_border_mesh = build_world_border_mesh(&self.world_border_render_state);
        let has_world_border_buffers = world_border_mesh.as_ref().is_some_and(|mesh| {
            self.frame_world_border_vertices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&mesh.vertices),
            ) && self.frame_world_border_indices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&mesh.indices),
            )
        });
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
            if let (Some(mesh), true) = (&lightning_mesh, has_lightning_buffers) {
                let vertex_buffer = self.frame_lightning_vertices.buffer().expect("uploaded");
                let index_buffer = self.frame_lightning_indices.buffer().expect("uploaded");
                pass.set_pipeline(&self.lightning_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                stats.weather_draw_calls += 1;
            }
            if let (Some(mesh), true) = (&weather_mesh, has_weather_buffers) {
                let vertex_buffer = self.frame_weather_vertices.buffer().expect("uploaded");
                let index_buffer = self.frame_weather_indices.buffer().expect("uploaded");
                pass.set_pipeline(&self.weather_pipeline);
                stats.pipeline_switches += 1;
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                if !mesh.rain_indices.is_empty() {
                    if let Some(texture) = &self.weather_rain_texture {
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.draw_indexed(mesh.rain_indices.clone(), 0, 0..1);
                        stats.weather_draw_calls += 1;
                    }
                }
                if !mesh.snow_indices.is_empty() {
                    if let Some(texture) = &self.weather_snow_texture {
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.draw_indexed(mesh.snow_indices.clone(), 0, 0..1);
                        stats.weather_draw_calls += 1;
                    }
                }
            }
            // Vanilla `LevelRenderer.addWeatherPass` runs
            // `worldBorderRenderer.render(...)` after
            // `weatherEffectRenderer.render(...)` (LevelRenderer.java:751-758),
            // and the border draws into the weather target's color+depth when
            // that target exists (WorldBorderRenderer.java:143-150). The mesh
            // indices already carry the closest-face-first sorted visible-side
            // draw list, so one indexed draw replays vanilla's
            // `drawMultipleIndexed` order (WorldBorderRenderer.java:176-183).
            if let (Some(mesh), true) = (&world_border_mesh, has_world_border_buffers) {
                if let Some(texture) = &self.world_border_texture {
                    let vertex_buffer =
                        self.frame_world_border_vertices.buffer().expect("uploaded");
                    let index_buffer = self.frame_world_border_indices.buffer().expect("uploaded");
                    pass.set_pipeline(&self.world_border_pipeline);
                    stats.pipeline_switches += 1;
                    pass.set_bind_group(0, &texture.bind_group, &[]);
                    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                    stats.weather_draw_calls += 1;
                }
            }
        }
    }

    fn transparency_combine_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(TRANSPARENCY_COMBINE_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.transparency_final_target.view,
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
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.transparency_combine_bind_group.bind_group, &[]);
            pass.draw(0..3, 0..1);
            stats.transparency_combine_draw_calls += 1;
        }
    }

    fn transparency_blit_pass(
        &self,
        frame: &FrameTarget,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let surface_view = frame
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(TRANSPARENCY_BLIT_PASS_LABEL),
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
            pass.set_pipeline(&self.transparency_blit_pipeline);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.transparency_final_target.bind_group, &[]);
            pass.draw(0..3, 0..1);
            stats.transparency_blit_draw_calls += 1;
        }
    }

    fn first_person_item_pass(
        &self,
        frame: &FrameTarget,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let (block_vertices, block_indices) = self.collect_first_person_block_item_model_geometry();
        let block_buffers = self.create_item_model_frame_buffers(&block_vertices, &block_indices);
        let (block_translucent_vertices, block_translucent_indices) =
            self.collect_first_person_block_item_model_translucent_geometry();
        let block_translucent_buffers = self.create_item_model_frame_buffers(
            &block_translucent_vertices,
            &block_translucent_indices,
        );
        let (flat_vertices, flat_indices) = self.collect_first_person_flat_item_model_geometry();
        let flat_buffers = self.create_item_model_frame_buffers(&flat_vertices, &flat_indices);
        let (flat_translucent_vertices, flat_translucent_indices) =
            self.collect_first_person_flat_item_model_translucent_geometry();
        let flat_translucent_buffers = self
            .create_item_model_frame_buffers(&flat_translucent_vertices, &flat_translucent_indices);
        let (map_background_vertices, map_background_indices) =
            self.collect_first_person_map_background_geometry();
        let map_background_buffers =
            self.create_item_model_frame_buffers(&map_background_vertices, &map_background_indices);
        let (map_vertices, map_indices) = self.collect_first_person_map_geometry();
        let map_buffers = self.create_item_model_frame_buffers(&map_vertices, &map_indices);
        let (map_decoration_vertices, map_decoration_indices) =
            self.collect_first_person_map_decoration_geometry();
        let map_decoration_buffers =
            self.create_item_model_frame_buffers(&map_decoration_vertices, &map_decoration_indices);
        let (map_text_vertices, map_text_indices) = self.collect_first_person_map_text_geometry();
        let map_text_buffers =
            self.create_item_model_frame_buffers(&map_text_vertices, &map_text_indices);
        let (glint_vertices, glint_indices) = self.collect_first_person_item_model_glint_geometry();
        let glint_buffers = self.create_item_model_frame_buffers(&glint_vertices, &glint_indices);
        let (glint_translucent_vertices, glint_translucent_indices) =
            self.collect_first_person_item_model_glint_translucent_geometry();
        let glint_translucent_buffers = self.create_item_model_frame_buffers(
            &glint_translucent_vertices,
            &glint_translucent_indices,
        );

        if block_buffers.is_none()
            && block_translucent_buffers.is_none()
            && flat_buffers.is_none()
            && flat_translucent_buffers.is_none()
            && map_background_buffers.is_none()
            && map_buffers.is_none()
            && map_decoration_buffers.is_none()
            && map_text_buffers.is_none()
            && glint_buffers.is_none()
            && glint_translucent_buffers.is_none()
            && self.first_person_player_arm_mesh.is_none()
            && self.first_person_dynamic_player_arm_mesh.is_none()
        {
            return;
        }

        let surface_view = frame
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(FIRST_PERSON_ITEM_PASS_LABEL),
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

        if let (Some(mesh), Some(atlas)) = (
            &self.first_person_player_arm_mesh,
            &self.entity_model_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            stats.entity_model_draw_calls += 1;
        }
        if let (Some(mesh), Some(atlas)) = (
            &self.first_person_dynamic_player_arm_mesh,
            &self.entity_dynamic_player_skin_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            stats.entity_model_draw_calls += 1;
        }

        if let Some(buffers) = &block_buffers {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_pipeline,
                buffers,
                &self.terrain_bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(atlas), Some(buffers)) = (&self.item_entity_atlas, &flat_buffers) {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_pipeline,
                buffers,
                &atlas.bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(atlas), Some(buffers)) = (
            &self.first_person_map_background_atlas,
            &map_background_buffers,
        ) {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_pipeline,
                buffers,
                &atlas.bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(atlas), Some(buffers)) = (&self.item_frame_map_atlas, &map_buffers) {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_pipeline,
                buffers,
                &atlas.bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(atlas), Some(buffers)) = (
            &self.item_frame_map_decoration_atlas,
            &map_decoration_buffers,
        ) {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_pipeline,
                buffers,
                &atlas.bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(atlas), Some(buffers)) =
            (&self.item_frame_map_text_font_atlas, &map_text_buffers)
        {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_pipeline,
                buffers,
                &atlas.bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(glint), Some(buffers)) = (&self.item_glint_texture, &glint_buffers) {
            self.draw_item_model_glint_frame_buffers(&mut pass, buffers, &glint.main_bind_group);
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let Some(buffers) = &block_translucent_buffers {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_translucent_pipeline,
                buffers,
                &self.terrain_bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(atlas), Some(buffers)) = (&self.item_entity_atlas, &flat_translucent_buffers) {
            self.draw_item_model_frame_buffers(
                &mut pass,
                &self.item_model_translucent_pipeline,
                buffers,
                &atlas.bind_group,
            );
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
        if let (Some(glint), Some(buffers)) = (&self.item_glint_texture, &glint_translucent_buffers)
        {
            self.draw_item_model_glint_frame_buffers(&mut pass, buffers, &glint.main_bind_group);
            stats.pipeline_switches += 1;
            stats.item_model_draw_calls += 1;
        }
    }

    /// GUI entity picture-in-picture targets (vanilla `GuiRenderer.preparePictureInPicture` →
    /// `PictureInPictureRenderer.prepare` + `GuiEntityRenderer`): before the GUI draws, each
    /// sanitized `HudEntityPreview` renders through the entity model pipelines into its own
    /// persistent color+depth PIP target — cleared per preview, GUI-ortho `hud_entity_preview_pip`
    /// camera under `ENTITY_IN_UI` lighting, depth fully isolated from the world depth target.
    /// `collect_hud_draws` then blits each target's color texture into the HUD frame in GUI
    /// submission order (background, then preview blit, then slot items / overlays), matching
    /// vanilla `blitTexture`'s `addBlitToCurrentLayer`.
    fn entity_preview_pip_passes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let previews = match &self.hud_inventory_screen {
            Some(screen) if !screen.entity_previews.is_empty() => screen.entity_previews.clone(),
            _ => {
                self.hud_entity_preview_pip_targets.clear();
                return;
            }
        };
        let mut retired = std::mem::take(&mut self.hud_entity_preview_pip_targets);
        retired.truncate(previews.len());
        let mut reusable = retired.into_iter();
        let mut targets = Vec::with_capacity(previews.len());
        for preview in &previews {
            let mut target = self.ensure_hud_entity_preview_pip_target(
                reusable.next(),
                preview.rect.width,
                preview.rect.height,
            );
            let (draw_calls, pipeline_switches) =
                self.encode_hud_entity_preview_pip(encoder, &mut target, preview);
            stats.entity_model_draw_calls += draw_calls;
            stats.pipeline_switches += pipeline_switches;
            targets.push(target);
        }
        self.hud_entity_preview_pip_targets = targets;
    }

    fn hud_passes(
        &mut self,
        frame: &FrameTarget,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        let surface_view = frame
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        // `collect_hud_draws` borrows self for the lifetime of its commands, so
        // temporarily move the persistent buffer out to upload alongside them.
        let mut frame_hud_vertices = std::mem::replace(
            &mut self.frame_hud_vertices,
            FrameDataBuffer::vertex("bbb-hud-frame-vertices"),
        );
        let hud_draws = self.collect_hud_draws();
        let has_hud_vertices = !hud_draws.commands.is_empty()
            && frame_hud_vertices.upload(
                &self.device,
                &self.queue,
                bytemuck::cast_slice(&hud_draws.vertices),
            );
        if has_hud_vertices {
            let hud_vertex_buffer = frame_hud_vertices.buffer().expect("uploaded");
            let hud_commands = &hud_draws.commands[..hud_draws.post_gui_item_start];
            if !hud_commands.is_empty() {
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
                pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                let (draw_calls, switches) = self.draw_hud_commands(&mut pass, hud_commands);
                stats.hud_draw_calls = draw_calls;
                stats.pipeline_switches += switches;
            }
        }

        // GUI 3D block-item icons: the hotbar's block items render as 3D models (vanilla inventory item
        // rendering) under the GUI ortho camera, after the 2D HUD backgrounds / flat items and before
        // item decorations / foreground overlays. A freshly-cleared depth buffer isolates the 3D icon
        // faces within each slot. Block-light items sample the blocks atlas via the GUI item bind group
        // (the world camera's pass already finished, so reusing the depth target with a clear is safe).
        {
            let gui_item_meshes = self.collect_hud_block_item_mesh();
            if !gui_item_meshes.is_empty() {
                let solid_buffers = self.create_item_model_frame_buffers(
                    &gui_item_meshes.solid.vertices,
                    &gui_item_meshes.solid.indices,
                );
                let glint_buffers = self.create_item_model_frame_buffers(
                    &gui_item_meshes.glint.vertices,
                    &gui_item_meshes.glint.indices,
                );
                let translucent_buffers = self.create_item_model_frame_buffers(
                    &gui_item_meshes.translucent.vertices,
                    &gui_item_meshes.translucent.indices,
                );
                let glint_translucent_buffers = self.create_item_model_frame_buffers(
                    &gui_item_meshes.glint_translucent.vertices,
                    &gui_item_meshes.glint_translucent.indices,
                );
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
                if let Some(buffers) = &solid_buffers {
                    self.draw_item_model_frame_buffers(
                        &mut pass,
                        &self.item_model_pipeline,
                        buffers,
                        &self.gui_item_bind_group,
                    );
                    stats.pipeline_switches += 1;
                    stats.item_model_draw_calls += 1;
                }
                if let (Some(glint), Some(buffers)) = (&self.item_glint_texture, &glint_buffers) {
                    self.draw_item_model_glint_frame_buffers(
                        &mut pass,
                        buffers,
                        &glint.gui_bind_group,
                    );
                    stats.pipeline_switches += 1;
                    stats.item_model_draw_calls += 1;
                }
                if let Some(buffers) = &translucent_buffers {
                    self.draw_item_model_frame_buffers(
                        &mut pass,
                        &self.item_model_translucent_pipeline,
                        buffers,
                        &self.gui_item_bind_group,
                    );
                    stats.pipeline_switches += 1;
                    stats.item_model_draw_calls += 1;
                }
                if let (Some(glint), Some(buffers)) =
                    (&self.item_glint_texture, &glint_translucent_buffers)
                {
                    self.draw_item_model_glint_frame_buffers(
                        &mut pass,
                        buffers,
                        &glint.gui_bind_group,
                    );
                    stats.pipeline_switches += 1;
                    stats.item_model_draw_calls += 1;
                }
            }
        }

        if has_hud_vertices {
            let hud_vertex_buffer = frame_hud_vertices.buffer().expect("uploaded");
            let hud_commands = &hud_draws.commands[hud_draws.post_gui_item_start..];
            if !hud_commands.is_empty() {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-native-hud-overlay-pass"),
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
                pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                let (draw_calls, switches) = self.draw_hud_commands(&mut pass, hud_commands);
                stats.hud_draw_calls += draw_calls;
                stats.pipeline_switches += switches;
            }
        }
        drop(hud_draws);
        self.frame_hud_vertices = frame_hud_vertices;
    }

    fn finish_frame(
        &mut self,
        mut encoder: wgpu::CommandEncoder,
        frame: FrameTarget,
        screenshot: Option<&Path>,
        stats: FrameDrawStats,
    ) -> Result<()> {
        let readback = match screenshot {
            Some(path) => Some((
                self.prepare_screenshot_copy(&mut encoder, frame.texture())?,
                path,
            )),
            None => None,
        };

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        if let Some((readback, path)) = readback {
            self.finish_screenshot(readback, path)?;
            self.counters.screenshots_written += 1;
        }

        self.counters.frame_index += 1;
        self.counters.opaque_draw_calls = stats.opaque_draw_calls;
        self.counters.cutout_draw_calls = stats.cutout_draw_calls;
        self.counters.translucent_draw_calls = stats.translucent_draw_calls;
        self.counters.block_destroy_overlay_draw_calls = stats.block_destroy_overlay_draw_calls;
        self.counters.sky_draw_calls = stats.sky_draw_calls;
        self.counters.particle_draw_calls = stats.particle_draw_calls;
        self.counters.weather_draw_calls = stats.weather_draw_calls;
        self.counters.item_entity_draw_calls = stats.item_entity_draw_calls;
        self.counters.selection_draw_calls = stats.selection_draw_calls;
        self.counters.entity_scene_draw_calls =
            stats.entity_scene_draw_calls + stats.entity_model_draw_calls;
        self.counters.entity_target_draw_calls = stats.entity_target_draw_calls;
        self.counters.hud_draw_calls = stats.hud_draw_calls;
        self.counters.draw_calls = stats.opaque_draw_calls
            + stats.cutout_draw_calls
            + stats.translucent_draw_calls
            + stats.block_destroy_overlay_draw_calls
            + stats.sky_draw_calls
            + stats.entity_model_draw_calls
            + stats.outline_composite_draw_calls
            + stats.transparency_combine_draw_calls
            + stats.transparency_blit_draw_calls
            + stats.particle_draw_calls
            + stats.weather_draw_calls
            + stats.item_entity_draw_calls
            + stats.item_model_draw_calls
            + stats.selection_draw_calls
            + stats.entity_scene_draw_calls
            + stats.entity_target_draw_calls
            + stats.hud_draw_calls
            + stats.lightmap_draw_calls;
        self.counters.pipeline_switches = stats.pipeline_switches;
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
        self.draw_item_model_geometry_with_pipeline(
            encoder,
            view,
            vertices,
            indices,
            bind_group,
            &self.item_model_pipeline,
        );
    }

    fn draw_item_model_geometry_with_pipeline(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        vertices: &[crate::item_models::ItemModelVertex],
        indices: &[u32],
        bind_group: &wgpu::BindGroup,
        pipeline: &wgpu::RenderPipeline,
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
        self.draw_item_model_frame_buffers(&mut pass, pipeline, &buffers, bind_group);
    }

    fn draw_item_model_glint_geometry(
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
            label: Some("bbb-native-item-model-glint-pass"),
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
        self.draw_item_model_glint_frame_buffers(&mut pass, &buffers, bind_group);
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

    fn draw_particle_vertex_batch<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        vertex_buffer: &'a wgpu::Buffer,
        batch: &'a ParticlePipelineVertexBatch,
        stats: &mut FrameDrawStats,
    ) {
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        for draw in &batch.draws {
            let Some(bind_group) = self.particle_texture_atlas_bind_group(*draw) else {
                continue;
            };
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.draw(draw.vertex_start..draw.vertex_end(), 0..1);
            stats.particle_draw_calls += 1;
        }
    }

    fn particle_texture_atlas_bind_group(
        &self,
        draw: ParticleAtlasDrawRange,
    ) -> Option<&wgpu::BindGroup> {
        match draw.texture_atlas {
            ParticleTextureAtlasKind::Particles => {
                self.particle_atlas.as_ref().map(|atlas| &atlas.bind_group)
            }
            ParticleTextureAtlasKind::Terrain => Some(&self.terrain_bind_group),
            ParticleTextureAtlasKind::Items => self
                .item_entity_atlas
                .as_ref()
                .map(|atlas| &atlas.bind_group),
        }
    }

    fn draw_item_model_glint_frame_buffers<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        buffers: &'a ItemModelFrameBuffers,
        bind_group: &'a wgpu::BindGroup,
    ) {
        pass.set_pipeline(&self.item_model_glint_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
        pass.set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..buffers.index_count, 0, 0..1);
    }

    fn draw_hud_commands<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        commands: &'a [crate::hud::HudDrawCommand<'a>],
    ) -> (u64, u64) {
        let mut active_pipeline = None;
        let mut draw_calls = 0;
        let mut pipeline_switches = 0;
        for command in commands {
            match command {
                crate::hud::HudDrawCommand::Sprite { sprite, start, end } => {
                    if active_pipeline != Some(HudActivePipeline::Sprite) {
                        pass.set_pipeline(&self.hud_pipeline);
                        active_pipeline = Some(HudActivePipeline::Sprite);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, &sprite.bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                crate::hud::HudDrawCommand::ItemGlint { mask, start, end } => {
                    let Some(glint) = &self.item_glint_texture else {
                        continue;
                    };
                    if active_pipeline != Some(HudActivePipeline::ItemGlint) {
                        pass.set_pipeline(&self.hud_item_glint_pipeline);
                        pass.set_bind_group(0, &glint.gui_bind_group, &[]);
                        active_pipeline = Some(HudActivePipeline::ItemGlint);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(1, &mask.bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
                // Vanilla `PictureInPictureRenderer.blitTexture`: the preview's private PIP color
                // texture draws as a GUI-textured quad on the current layer. The PIP pass rendered
                // the texture earlier this frame (`entity_preview_pip_passes`).
                crate::hud::HudDrawCommand::EntityPreviewBlit {
                    target_index,
                    start,
                    end,
                } => {
                    let Some(target) = self.hud_entity_preview_pip_targets.get(*target_index)
                    else {
                        continue;
                    };
                    if active_pipeline != Some(HudActivePipeline::Sprite) {
                        pass.set_pipeline(&self.hud_pipeline);
                        active_pipeline = Some(HudActivePipeline::Sprite);
                        pipeline_switches += 1;
                    }
                    pass.set_bind_group(0, &target.blit_bind_group, &[]);
                    pass.draw(*start..*end, 0..1);
                    draw_calls += 1;
                }
            }
        }
        (draw_calls, pipeline_switches)
    }

    fn has_entity_translucent_features(&self) -> bool {
        (self.entity_model_texture_atlas.is_some()
            && (self.entity_model_translucent_mesh.is_some()
                || self.entity_model_armor_translucent_mesh.is_some()
                || self.entity_model_translucent_emissive_mesh.is_some()
                || self.entity_model_eyes_mesh.is_some()
                || self.entity_model_dragon_rays_mesh.is_some()
                || self.entity_model_dragon_rays_depth_mesh.is_some()
                || self.entity_model_end_portal_mesh.is_some()
                || self.entity_model_end_gateway_mesh.is_some()
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
        if !self.entity_model_sorted_item_entity_draws.is_empty() {
            for draw in &self.entity_model_sorted_item_entity_draws {
                self.draw_entity_textured_range(
                    pass,
                    *draw,
                    true,
                    pipeline_switches,
                    entity_model_draw_calls,
                );
            }
            return;
        }

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
            &self.entity_model_item_entity_translucent_cull_mesh,
            &self.entity_model_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_cull_pipeline);
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
            &self.entity_dynamic_player_skin_item_entity_translucent_cull_mesh,
            &self.entity_dynamic_player_skin_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_cull_pipeline);
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
        if let (Some(mesh), Some(atlas)) = (
            &self.entity_dynamic_player_texture_item_entity_translucent_cull_mesh,
            &self.entity_dynamic_player_texture_atlas,
        ) {
            pass.set_pipeline(&self.entity_model_translucent_cull_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
    }

    fn draw_entity_textured_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityModelTexturedDrawRange,
        item_entity_target: bool,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        let Some((mesh, bind_group)) =
            self.entity_textured_range_resources(draw, item_entity_target)
        else {
            return;
        };
        let index_end = draw.index_start.saturating_add(draw.index_count);
        if draw.index_count == 0 || index_end > mesh.index_count {
            return;
        }

        let uses_translucent_emissive_pipeline =
            draw.render_type == EntityModelLayerRenderType::EntityTranslucentEmissive;
        let uses_armor_translucent_pipeline =
            draw.render_type == EntityModelLayerRenderType::ArmorTranslucent;
        let uses_eyes_pipeline = draw.render_type == EntityModelLayerRenderType::Eyes;
        if uses_translucent_emissive_pipeline {
            pass.set_pipeline(&self.entity_model_translucent_emissive_pipeline);
        } else if uses_armor_translucent_pipeline {
            pass.set_pipeline(&self.entity_model_armor_translucent_pipeline);
        } else if uses_eyes_pipeline {
            pass.set_pipeline(&self.entity_model_eyes_pipeline);
        } else if draw.surface_cull {
            pass.set_pipeline(&self.entity_model_translucent_cull_pipeline);
        } else {
            pass.set_pipeline(&self.entity_model_translucent_pipeline);
        }
        *pipeline_switches += 1;
        pass.set_bind_group(0, bind_group, &[]);
        if !uses_translucent_emissive_pipeline && !uses_eyes_pipeline {
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
        }
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(draw.index_start..index_end, 0, 0..1);
        *entity_model_draw_calls += 1;
    }

    fn entity_textured_range_resources<'a>(
        &'a self,
        draw: EntityModelTexturedDrawRange,
        item_entity_target: bool,
    ) -> Option<(&'a EntityModelTexturedMeshGpu, &'a wgpu::BindGroup)> {
        if draw.render_type == EntityModelLayerRenderType::EntityTranslucentEmissive {
            if item_entity_target
                || draw.atlas != EntityModelTexturedDrawAtlas::Static
                || draw.surface_cull
            {
                return None;
            }
            return Some((
                self.entity_model_translucent_emissive_mesh.as_ref()?,
                &self.entity_model_texture_atlas.as_ref()?.bind_group,
            ));
        }
        if draw.render_type == EntityModelLayerRenderType::ArmorTranslucent {
            if item_entity_target
                || draw.atlas != EntityModelTexturedDrawAtlas::Static
                || draw.surface_cull
            {
                return None;
            }
            return Some((
                self.entity_model_armor_translucent_mesh.as_ref()?,
                &self.entity_model_texture_atlas.as_ref()?.bind_group,
            ));
        }
        if draw.render_type == EntityModelLayerRenderType::Eyes {
            if item_entity_target
                || draw.atlas != EntityModelTexturedDrawAtlas::Static
                || draw.surface_cull
            {
                return None;
            }
            return Some((
                self.entity_model_eyes_mesh.as_ref()?,
                &self.entity_model_texture_atlas.as_ref()?.bind_group,
            ));
        }
        match (draw.atlas, item_entity_target, draw.surface_cull) {
            (EntityModelTexturedDrawAtlas::Static, false, false) => Some((
                self.entity_model_translucent_mesh.as_ref()?,
                &self.entity_model_texture_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerSkin, false, false) => Some((
                self.entity_dynamic_player_skin_translucent_mesh.as_ref()?,
                &self.entity_dynamic_player_skin_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerTexture, false, false) => Some((
                self.entity_dynamic_player_texture_translucent_mesh
                    .as_ref()?,
                &self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()?
                    .bind_group,
            )),
            (EntityModelTexturedDrawAtlas::Static, true, false) => Some((
                self.entity_model_item_entity_translucent_mesh.as_ref()?,
                &self.entity_model_texture_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::Static, true, true) => Some((
                self.entity_model_item_entity_translucent_cull_mesh
                    .as_ref()?,
                &self.entity_model_texture_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerSkin, true, false) => Some((
                self.entity_dynamic_player_skin_item_entity_translucent_mesh
                    .as_ref()?,
                &self.entity_dynamic_player_skin_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerSkin, true, true) => Some((
                self.entity_dynamic_player_skin_item_entity_translucent_cull_mesh
                    .as_ref()?,
                &self.entity_dynamic_player_skin_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerTexture, true, false) => Some((
                self.entity_dynamic_player_texture_item_entity_translucent_mesh
                    .as_ref()?,
                &self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()?
                    .bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerTexture, true, true) => Some((
                self.entity_dynamic_player_texture_item_entity_translucent_cull_mesh
                    .as_ref()?,
                &self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()?
                    .bind_group,
            )),
            _ => None,
        }
    }

    fn draw_entity_main_translucent_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityModelTranslucentDrawRange,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        match draw {
            EntityModelTranslucentDrawRange::Textured(draw) => self.draw_entity_textured_range(
                pass,
                draw,
                false,
                pipeline_switches,
                entity_model_draw_calls,
            ),
            EntityModelTranslucentDrawRange::Scroll(draw)
            | EntityModelTranslucentDrawRange::AdditiveScroll(draw) => self
                .draw_entity_scroll_range(pass, draw, pipeline_switches, entity_model_draw_calls),
            EntityModelTranslucentDrawRange::PositionColor(draw) => self
                .draw_entity_position_color_range(
                    pass,
                    draw,
                    pipeline_switches,
                    entity_model_draw_calls,
                ),
        }
    }

    fn draw_entity_position_color_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityModelPositionColorDrawRange,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        let Some((mesh, pipeline)) = self.entity_position_color_range_resources(draw) else {
            return;
        };
        let index_end = draw.index_start.saturating_add(draw.index_count);
        if draw.index_count == 0 || index_end > mesh.index_count {
            return;
        }
        pass.set_pipeline(pipeline);
        *pipeline_switches += 1;
        pass.set_bind_group(0, &self.terrain_bind_group, &[]);
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(draw.index_start..index_end, 0, 0..1);
        *entity_model_draw_calls += 1;
    }

    fn entity_position_color_range_resources<'a>(
        &'a self,
        draw: EntityModelPositionColorDrawRange,
    ) -> Option<(&'a EntityModelMeshGpu, &'a wgpu::RenderPipeline)> {
        match draw.render_type {
            EntityModelLayerRenderType::DragonRays => Some((
                self.entity_model_dragon_rays_mesh.as_ref()?,
                &self.entity_model_dragon_rays_pipeline,
            )),
            EntityModelLayerRenderType::DragonRaysDepth => Some((
                self.entity_model_dragon_rays_depth_mesh.as_ref()?,
                &self.entity_model_dragon_rays_depth_pipeline,
            )),
            EntityModelLayerRenderType::EndPortal => Some((
                self.entity_model_end_portal_mesh.as_ref()?,
                &self.entity_model_dragon_rays_pipeline,
            )),
            EntityModelLayerRenderType::EndGateway => Some((
                self.entity_model_end_gateway_mesh.as_ref()?,
                &self.entity_model_dragon_rays_pipeline,
            )),
            _ => None,
        }
    }

    fn draw_entity_scroll_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityModelScrollDrawRange,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        let (mesh, uses_lightmap) = match draw.render_type {
            EntityModelLayerRenderType::BreezeWind
            | EntityModelLayerRenderType::EndCrystalBeam
            | EntityModelLayerRenderType::EndGatewayBeam => {
                let Some(mesh) = self.entity_model_scroll_mesh.as_ref() else {
                    return;
                };
                (mesh, true)
            }
            EntityModelLayerRenderType::EnergySwirl => {
                let Some(mesh) = self.entity_model_scroll_additive_mesh.as_ref() else {
                    return;
                };
                (mesh, false)
            }
            _ => return,
        };
        let index_end = draw.index_start.saturating_add(draw.index_count);
        if draw.index_count == 0 || index_end > mesh.index_count {
            return;
        }

        let Some(atlas) = &self.entity_model_texture_atlas else {
            return;
        };
        if draw.render_type == EntityModelLayerRenderType::EnergySwirl {
            pass.set_pipeline(&self.entity_model_scroll_additive_pipeline);
        } else {
            pass.set_pipeline(&self.entity_model_scroll_pipeline);
        }
        *pipeline_switches += 1;
        pass.set_bind_group(0, &atlas.bind_group, &[]);
        if uses_lightmap {
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
        }
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(draw.index_start..index_end, 0, 0..1);
        *entity_model_draw_calls += 1;
    }

    fn draw_entity_translucent_features<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        if !self.entity_model_sorted_main_translucent_draws.is_empty() {
            for draw in &self.entity_model_sorted_main_translucent_draws {
                self.draw_entity_main_translucent_range(
                    pass,
                    *draw,
                    pipeline_switches,
                    entity_model_draw_calls,
                );
            }
            return;
        }
        if self.entity_model_sorted_translucent_draws.is_empty() {
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
                &self.entity_model_armor_translucent_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_armor_translucent_pipeline);
                *pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                *entity_model_draw_calls += 1;
            }
            if let (Some(mesh), Some(atlas)) = (
                &self.entity_model_translucent_emissive_mesh,
                &self.entity_model_texture_atlas,
            ) {
                pass.set_pipeline(&self.entity_model_translucent_emissive_pipeline);
                *pipeline_switches += 1;
                pass.set_bind_group(0, &atlas.bind_group, &[]);
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
        } else {
            for draw in &self.entity_model_sorted_translucent_draws {
                self.draw_entity_textured_range(
                    pass,
                    *draw,
                    false,
                    pipeline_switches,
                    entity_model_draw_calls,
                );
            }
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
        if let Some(mesh) = &self.entity_model_dragon_rays_mesh {
            pass.set_pipeline(&self.entity_model_dragon_rays_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let Some(mesh) = &self.entity_model_dragon_rays_depth_mesh {
            pass.set_pipeline(&self.entity_model_dragon_rays_depth_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let Some(mesh) = &self.entity_model_end_portal_mesh {
            pass.set_pipeline(&self.entity_model_dragon_rays_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            *entity_model_draw_calls += 1;
        }
        if let Some(mesh) = &self.entity_model_end_gateway_mesh {
            pass.set_pipeline(&self.entity_model_dragon_rays_pipeline);
            *pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
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

    /// The frame's step sequence in submission order. `render()` calls exactly
    /// these methods in exactly this order, and `render_steps_are_defined_in_frame_
    /// execution_order` pins each step's definition to the same order, so source
    /// position in this file tracks frame execution order.
    const FRAME_STEPS: &[&str] = &[
        "lightmap_pass",
        "main_world_pass",
        "opaque_particle_main_pass",
        "copy_main_depth_to_feature_targets",
        "entity_translucent_feature_pass",
        "item_entity_target_pass",
        "block_destroy_overlay_pass",
        "entity_outline_target_pass",
        "translucent_target_pass",
        "item_entity_line_target_pass",
        "particle_target_pass",
        "entity_outline_post_chain",
        "clouds_pass",
        "weather_target_pass",
        "transparency_combine_pass",
        "transparency_blit_pass",
        "first_person_item_pass",
        "entity_preview_pip_passes",
        "hud_passes",
        "finish_frame",
    ];

    #[test]
    fn render_steps_are_defined_in_frame_execution_order() {
        let source = include_str!("render.rs");
        let render_start = source
            .find("pub fn render(")
            .expect("render entry point is present");
        let render_end = render_start
            + source[render_start..]
                .find("\n    }")
                .expect("render entry point ends");
        let render_body = &source[render_start..render_end];

        // render() encodes no GPU work directly; every pass, copy, and the
        // submit/present tail live in the ordered step methods.
        assert!(
            !render_body.contains("begin_render_pass")
                && !render_body.contains("copy_texture_to_texture")
                && !render_body.contains("queue.submit"),
            "render() stays a pure step orchestrator"
        );

        // Each step is called exactly once from render(), in FRAME_STEPS order.
        let mut previous_call = 0;
        for step in FRAME_STEPS {
            let call = format!("self.{step}(");
            let position = render_body
                .find(&call)
                .unwrap_or_else(|| panic!("{step} is called from render()"));
            assert_eq!(
                render_body.matches(&call).count(),
                1,
                "{step} is called exactly once per frame"
            );
            assert!(
                position > previous_call,
                "{step} is called in FRAME_STEPS order"
            );
            previous_call = position;
        }

        // Each step is defined after render(), in call order, so the source
        // positions the other tests in this module compare reflect real frame
        // execution order.
        let mut previous_definition = render_end;
        for step in FRAME_STEPS {
            let definition = source
                .find(&format!("fn {step}("))
                .unwrap_or_else(|| panic!("{step} is defined in render.rs"));
            assert!(
                definition > previous_definition,
                "{step} is defined in frame execution order"
            );
            previous_definition = definition;
        }
    }

    #[test]
    fn render_calls_exactly_one_step_per_frame_steps_entry() {
        let source = include_str!("render.rs");
        let render_start = source
            .find("pub fn render(")
            .expect("render entry point is present");
        let render_end = render_start
            + source[render_start..]
                .find("\n    }")
                .expect("render entry point ends");
        let render_body = &source[render_start..render_end];

        // Count `self.<method>(` call sites in render()'s body. Every frame
        // step is invoked this way; other `self.` uses in render() (e.g.
        // `self.surface.acquire_frame(...)`, or `self\n    .device\n    \
        // .create_command_encoder(...)`) never match because a `.` or a
        // line break sits between the identifier and the `(`, so this stays
        // an exact count of step calls without needing a regex crate.
        let bytes = render_body.as_bytes();
        let mut step_call_count = 0;
        let mut search_from = 0;
        while let Some(offset) = render_body[search_from..].find("self.") {
            let ident_start = search_from + offset + "self.".len();
            let mut cursor = ident_start;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            if cursor > ident_start && bytes.get(cursor) == Some(&b'(') {
                step_call_count += 1;
            }
            search_from = ident_start;
        }

        // If this fails after adding a new pass to render(), the new step
        // method must also be added to FRAME_STEPS (in call order) so the
        // execution-order test above covers it.
        assert_eq!(
            step_call_count,
            FRAME_STEPS.len(),
            "render() must call exactly one step per FRAME_STEPS entry, and vice versa"
        );
    }

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
    fn first_person_item_pass_runs_after_world_composite_and_before_hud() {
        let source = include_str!("render.rs");
        let blit = source
            .find("self.transparency_blit_pass(&frame, &mut encoder, &mut stats);")
            .expect("world transparency blit is scheduled");
        let first_person = source
            .find("self.first_person_item_pass(&frame, &mut encoder, &mut stats);")
            .expect("first-person item pass is scheduled");
        let hud = source
            .find("self.hud_passes(&frame, &mut encoder, &mut stats);")
            .expect("HUD passes are scheduled");
        let pass = source
            .find("label: Some(FIRST_PERSON_ITEM_PASS_LABEL)")
            .expect("first-person item pass label is used");
        let depth_clear = source[pass..]
            .find("load: wgpu::LoadOp::Clear(1.0)")
            .map(|index| pass + index)
            .expect("first-person item pass clears depth");

        assert!(blit < first_person && first_person < hud);
        assert!(first_person < pass && pass < depth_clear);
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
                "pass.set_pipeline(&self.entity_model_textured_cull_pipeline)",
                "pass.set_bind_group(0, &atlas.bind_group, &[])",
                "textured cull entity",
            ),
            (
                "pass.set_pipeline(&self.entity_model_cutout_z_offset_pipeline)",
                "pass.set_bind_group(0, &atlas.bind_group, &[])",
                "textured z-offset entity",
            ),
            (
                "pass.set_pipeline(&self.entity_model_translucent_pipeline)",
                "pass.set_bind_group(0, &atlas.bind_group, &[])",
                "translucent entity",
            ),
            (
                "pass.set_pipeline(&self.entity_model_translucent_cull_pipeline)",
                "pass.set_bind_group(0, &atlas.bind_group, &[])",
                "translucent cull entity",
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
        let eyes_lightmap_guard = source[eyes..eyes_draw]
            .find("if !uses_translucent_emissive_pipeline && !uses_eyes_pipeline")
            .map(|index| eyes + index)
            .expect("eyes lightmap guard");
        let eyes_lightmap_bind = source[eyes_lightmap_guard..eyes_draw]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| eyes_lightmap_guard + index)
            .expect("guarded eyes lightmap bind");
        assert!(
            eyes < eyes_lightmap_guard
                && eyes_lightmap_guard < eyes_lightmap_bind
                && eyes_lightmap_bind < eyes_draw,
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
    fn item_model_glint_draws_after_solid_items_without_lightmap() {
        let source = include_str!("render.rs");
        let solid_items = source
            .find("let (flat_item_vertices, flat_item_indices)")
            .expect("flat solid item-model collection is present");
        let glint_collect = source[solid_items..]
            .find("let (item_model_glint_vertices, item_model_glint_indices)")
            .map(|index| solid_items + index)
            .expect("item-model glint collection follows solid item draws");
        let glint_draw = source[glint_collect..]
            .find("self.draw_item_model_glint_geometry(")
            .map(|index| glint_collect + index)
            .expect("item-model glint draw helper is called");
        let depth_copy = source[glint_draw..]
            .find("encoder.copy_texture_to_texture(")
            .map(|index| glint_draw + index)
            .expect("main depth copy follows item-model glint");

        assert!(
            solid_items < glint_collect && glint_collect < glint_draw && glint_draw < depth_copy,
            "solid item glint draws before target depth copies so depth-equal can match base items"
        );

        let helper = source
            .find("fn draw_item_model_glint_frame_buffers")
            .expect("item-model glint frame helper exists");
        let helper_end = source[helper..]
            .find("fn has_entity_translucent_features")
            .map(|index| helper + index)
            .expect("glint helper ends before next helper");
        assert!(
            !source[helper..helper_end].contains("lightmap.sample_bind_group"),
            "vanilla core/glint does not bind or sample LightTexture"
        );
        assert!(source[helper..helper_end].contains("&self.item_model_glint_pipeline"));
    }

    #[test]
    fn item_model_glint_translucent_draws_in_item_entity_target_after_translucent_base() {
        let source = include_str!("render.rs");
        let collect = source
            .find(
                "let (item_model_glint_translucent_vertices, item_model_glint_translucent_indices)",
            )
            .expect("translucent item-model glint collection is present");
        let item_entity_pass = source[collect..]
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .map(|index| collect + index)
            .expect("itemEntity target pass follows glintTranslucent collection");
        let translucent_base = source[item_entity_pass..]
            .find("&self.item_model_translucent_pipeline")
            .map(|index| item_entity_pass + index)
            .expect("translucent item-model base draw uses itemEntity target");
        let glint_draw = source[translucent_base..]
            .find("self.draw_item_model_glint_frame_buffers(")
            .map(|index| translucent_base + index)
            .expect("glintTranslucent draw follows translucent item base");
        let item_billboard = source[glint_draw..]
            .find("pass.set_pipeline(&self.item_entity_pipeline)")
            .map(|index| glint_draw + index)
            .expect("item entity billboards follow item-model glintTranslucent");

        assert!(
            collect < item_entity_pass
                && item_entity_pass < translucent_base
                && translucent_base < glint_draw
                && glint_draw < item_billboard,
            "glintTranslucent draws in the itemEntity target after translucent item-model base depth writes"
        );
        assert!(
            !source[glint_draw..item_billboard].contains("lightmap.sample_bind_group"),
            "vanilla core/glint does not bind LightTexture for glintTranslucent"
        );
    }

    #[test]
    fn hud_flat_item_glint_uses_gui_glint_bind_group_and_item_atlas_mask() {
        let source = include_str!("render.rs");
        let helper = source
            .find("fn draw_hud_commands")
            .expect("HUD command draw helper exists");
        let glint_arm = source[helper..]
            .find("HudDrawCommand::ItemGlint")
            .map(|index| helper + index)
            .expect("HUD item glint commands are handled");
        let pipeline = source[glint_arm..]
            .find("pass.set_pipeline(&self.hud_item_glint_pipeline)")
            .map(|index| glint_arm + index)
            .expect("HUD item glint uses its dedicated pipeline");
        let glint_bind = source[pipeline..]
            .find("pass.set_bind_group(0, &glint.gui_bind_group, &[])")
            .map(|index| pipeline + index)
            .expect("HUD item glint uses the GUI camera glint bind group");
        let mask_bind = source[glint_bind..]
            .find("pass.set_bind_group(1, &mask.bind_group, &[])")
            .map(|index| glint_bind + index)
            .expect("HUD item glint binds the item atlas as an alpha mask");
        let draw = source[mask_bind..]
            .find("pass.draw(*start..*end, 0..1)")
            .map(|index| mask_bind + index)
            .expect("HUD item glint draws after binding glint and mask textures");

        assert!(pipeline < glint_bind && glint_bind < mask_bind && mask_bind < draw);
        assert!(
            !source[glint_arm..draw].contains("lightmap.sample_bind_group"),
            "vanilla core/glint does not bind LightTexture for GUI item glint"
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
        let world_item_z_offset_forward_models = source
            .find("let (block_item_z_offset_forward_vertices, block_item_z_offset_forward_indices)")
            .expect("z-offset-forward world item-model collection is present");
        let world_item_z_offset_forward_draw = source[world_item_z_offset_forward_models..]
            .find("&self.item_model_z_offset_forward_pipeline")
            .map(|index| world_item_z_offset_forward_models + index)
            .expect("z-offset-forward world item-model pipeline is drawn");
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
                && world_item_draw < world_item_z_offset_forward_models
                && world_item_z_offset_forward_models < world_item_z_offset_forward_draw
                && world_item_z_offset_forward_draw < copy_translucent
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
        let map_text_collect = source
            .find("let (map_text_vertices, map_text_indices)")
            .expect("item-frame map text collection is present");
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
        let block_destroy = source
            .find("label: Some(\"bbb-native-block-destroy-overlay-pass\")")
            .expect("block destroy overlay pass label is used");
        let translucent = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");
        let particle = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");

        assert!(
            solid_collect < copy_item_entity
                && copy_item_entity < map_text_collect
                && map_text_collect < block_collect
                && block_collect < flat_collect
                && flat_collect < target
                && target < block_draw
                && block_bind_group < flat_draw
                && flat_bind_group < billboards
                && billboards < block_destroy
                && block_destroy < translucent
                && billboards < particle,
            "vanilla FeatureRenderDispatcher draws text before ItemFeatureRenderer, then block/crumbling features, translucent terrain, and particles"
        );
    }

    #[test]
    fn item_pickup_item_models_and_experience_orbs_draw_before_elder_guardians() {
        let source = include_str!("render.rs");
        let target = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let translucent_particles = source[target..]
            .find("has_translucent_particles")
            .map(|index| target + index)
            .expect("translucent single-quad particles draw in particle target");
        let pickup_block = source[target..]
            .find("&item_pickup_block_buffers")
            .map(|index| target + index)
            .expect("item-pickup block item meshes draw in particle target");
        let pickup_flat = source[pickup_block..]
            .find("&item_pickup_flat_buffers")
            .map(|index| pickup_block + index)
            .expect("item-pickup flat item meshes draw in particle target");
        let experience_orb = source[pickup_flat..]
            .find("experience_orb_pickup_particle_index_count")
            .map(|index| pickup_flat + index)
            .expect("experience-orb pickup billboard draws in item-pickup group");
        let elder_guardian = source[experience_orb..]
            .find("elder_guardian_particle_index_count")
            .map(|index| experience_orb + index)
            .expect("elder guardian particle group follows item-pickup group");

        assert!(
            translucent_particles < pickup_block
                && pickup_block < pickup_flat
                && pickup_flat < experience_orb
                && experience_orb < elder_guardian,
            "vanilla particle group order is SINGLE_QUADS, ITEM_PICKUP, ELDER_GUARDIANS"
        );
    }

    #[test]
    fn sky_disc_draws_before_sunrise_and_celestial_layers() {
        let source = include_str!("render.rs");
        let sky = source
            .find("pass.set_pipeline(&self.sky_pipeline)")
            .expect("sky disc pipeline is drawn");
        let sunrise = source
            .find("pass.set_pipeline(&self.sunrise_sunset_pipeline)")
            .expect("sunrise/sunset pipeline is drawn");
        let celestial = source
            .find("pass.set_pipeline(&self.celestial_pipeline)")
            .expect("celestial pipeline is drawn");
        let stars = source
            .find("pass.set_pipeline(&self.star_pipeline)")
            .expect("star pipeline is drawn");

        assert!(
            sky < sunrise && sunrise < celestial && celestial < stars,
            "vanilla LevelRenderer draws sky disc, sunrise/sunset, then sun/moon/stars"
        );
    }

    #[test]
    fn sky_disc_draw_binds_color_modulator_uniform() {
        let source = include_str!("render.rs");
        let sky = source
            .find("pass.set_pipeline(&self.sky_pipeline)")
            .expect("sky disc pipeline is drawn");
        let dynamic = source[sky..]
            .find("pass.set_bind_group(1, &sky_disc.dynamic.bind_group, &[])")
            .map(|index| sky + index)
            .expect("sky disc binds DynamicTransforms-style ColorModulator uniform");
        let vertex = source[dynamic..]
            .find("pass.set_vertex_buffer(0, vertex_buffer.slice(..))")
            .map(|index| dynamic + index)
            .expect("sky disc position vertex buffer is bound after dynamic uniform");
        let draw = source[vertex..]
            .find("pass.draw(0..sky_disc.disc_vertex_count, 0..1)")
            .map(|index| vertex + index)
            .expect("sky disc is drawn after binding dynamic uniform");

        assert!(
            sky < dynamic && dynamic < vertex && vertex < draw,
            "vanilla SKY uses core/sky with ColorModulator rather than per-vertex color"
        );
    }

    #[test]
    fn stars_draw_binds_color_modulator_uniform() {
        let source = include_str!("render.rs");
        let stars = source
            .find("pass.set_pipeline(&self.star_pipeline)")
            .expect("star pipeline is drawn");
        let dynamic = source[stars..]
            .find("pass.set_bind_group(1, &stars.dynamic.bind_group, &[])")
            .map(|index| stars + index)
            .expect("stars bind DynamicTransforms-style ColorModulator uniform");
        let draw = source[dynamic..]
            .find("pass.draw(0..stars.vertex_count, 0..1)")
            .map(|index| dynamic + index)
            .expect("stars are drawn after binding dynamic uniform");

        assert!(
            stars < dynamic && dynamic < draw,
            "vanilla STARS uses core/stars with ColorModulator rather than per-vertex color"
        );
    }

    #[test]
    fn end_sky_draw_binds_color_modulator_uniform() {
        let source = include_str!("render.rs");
        let end_sky = source
            .find("pass.set_pipeline(&self.end_sky_pipeline)")
            .expect("end sky pipeline is drawn");
        let texture = source[end_sky..]
            .find("pass.set_bind_group(1, &end_sky_texture.bind_group, &[])")
            .map(|index| end_sky + index)
            .expect("end sky texture is bound");
        let dynamic = source[texture..]
            .find("pass.set_bind_group(2, &self.end_sky_mesh.dynamic.bind_group, &[])")
            .map(|index| texture + index)
            .expect("end sky binds DynamicTransforms-style ColorModulator uniform");
        let draw = source[dynamic..]
            .find("pass.draw(0..self.end_sky_mesh.vertex_count, 0..1)")
            .map(|index| dynamic + index)
            .expect("end sky is drawn after binding dynamic uniform");

        assert!(
            end_sky < texture && texture < dynamic && dynamic < draw,
            "vanilla END_SKY uses core/position_tex_color with ColorModulator"
        );
    }

    #[test]
    fn celestial_draw_binds_color_modulator_uniform() {
        let source = include_str!("render.rs");
        let celestial = source
            .find("pass.set_pipeline(&self.celestial_pipeline)")
            .expect("celestial pipeline is drawn");
        let atlas = source[celestial..]
            .find("pass.set_bind_group(1, &celestial_atlas.bind_group, &[])")
            .map(|index| celestial + index)
            .expect("celestial atlas texture is bound");
        let sun_dynamic = source[atlas..]
            .find("pass.set_bind_group(2, &celestials.sun.dynamic.bind_group, &[])")
            .map(|index| atlas + index)
            .expect("sun binds DynamicTransforms-style ColorModulator uniform");
        let sun_draw = source[sun_dynamic..]
            .find("pass.draw(0..celestials.sun.vertex_count, 0..1)")
            .map(|index| sun_dynamic + index)
            .expect("sun is drawn after binding dynamic uniform");
        let moon_dynamic = source[sun_draw..]
            .find("pass.set_bind_group(2, &celestials.moon.dynamic.bind_group, &[])")
            .map(|index| sun_draw + index)
            .expect("moon binds DynamicTransforms-style ColorModulator uniform");
        let moon_draw = source[moon_dynamic..]
            .find("pass.draw(0..celestials.moon.vertex_count, 0..1)")
            .map(|index| moon_dynamic + index)
            .expect("moon is drawn after binding dynamic uniform");

        assert!(
            celestial < atlas
                && atlas < sun_dynamic
                && sun_dynamic < sun_draw
                && sun_draw < moon_dynamic
                && moon_dynamic < moon_draw,
            "vanilla CELESTIAL draws sun and moon with separate DynamicTransforms"
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
            .find("pass.set_pipeline(cloud_pipeline)")
            .map(|index| clouds + index)
            .expect("cloud pipeline is drawn in the cloud pass");
        let translucent = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");
        let item_entity_target = source
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .expect("item-entity target pass label is used");
        let item_entity_line_target = source
            .find("label: Some(ITEM_ENTITY_LINE_TARGET_PASS_LABEL)")
            .expect("item-entity line target pass label is used");
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
            entity_translucent_features < item_entity_target
                && entity_translucent_features < block_destroy
                && item_entity_target < block_destroy
                && block_destroy < translucent
                && translucent < item_entity_line_target
                && item_entity_line_target < particle_target
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
            source[item_entity_target..block_destroy].contains("view: item_entity_view"),
            "item-entity geometry writes the renderer-owned item_entity color target"
        );
        assert!(
            source[item_entity_target..block_destroy]
                .contains("view: &self.item_entity_target.depth.view"),
            "item-entity geometry writes the renderer-owned item_entity depth target"
        );
        assert!(
            source[item_entity_line_target..particle_target].contains("view: item_entity_view"),
            "line geometry appends to the renderer-owned item_entity color target before particles"
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
    fn translucent_terrain_draws_sections_back_to_front() {
        let source = include_str!("render.rs");
        let translucent_pipeline = source
            .find("pass.set_pipeline(&self.terrain_translucent_pipeline)")
            .expect("translucent terrain pipeline is selected");
        let draw = source[translucent_pipeline..]
            .find("pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1)")
            .map(|index| translucent_pipeline + index)
            .expect("translucent terrain sections are drawn");

        // The draw loop must index sections through the camera-sorted permutation
        // (`terrain_translucent_order`) rather than walking storage order, so
        // sections composite far→near like vanilla's reversed TRANSLUCENT draw
        // list (ChunkSectionsToRender.java:55-56, MC 26.1).
        assert!(
            source[translucent_pipeline..draw].contains("self.terrain_translucent_order"),
            "translucent terrain sections draw through the back-to-front section order"
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
            "pass.set_pipeline(&self.entity_model_armor_translucent_pipeline)",
            "pass.set_pipeline(&self.entity_model_eyes_pipeline)",
            "pass.set_pipeline(&self.entity_model_scroll_pipeline)",
            "pass.set_pipeline(&self.entity_model_scroll_additive_pipeline)",
        ] {
            assert!(
                source[helper..].contains(pipeline),
                "{pipeline} is emitted through the translucent feature helper"
            );
        }
        let scroll_additive = source[helper..]
            .find("pass.set_pipeline(&self.entity_model_scroll_additive_pipeline)")
            .map(|index| helper + index)
            .expect("energySwirl additive scroll pipeline is emitted");
        let helper_end = source[scroll_additive..]
            .find("\n    }\n}\n\n#[cfg(test)]")
            .map(|index| scroll_additive + index)
            .expect("entity translucent feature helper ends before tests");
        assert!(
            !source[scroll_additive..helper_end]
                .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])"),
            "vanilla ENERGY_SWIRL defines EMISSIVE, so the additive scroll draw does not bind LightTexture"
        );
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
    fn item_frame_map_text_draws_in_translucent_text_feature_phase() {
        let source = include_str!("render.rs");
        let copy_particles = depth_copy_to(source, "texture: &self.particle_target.depth._texture");
        let entity_pass = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let draw_features = source[entity_pass..]
            .find("self.draw_entity_translucent_features(")
            .map(|index| entity_pass + index)
            .expect("entity translucent feature helper is called from the pass");
        let map_text_collect = source[draw_features..]
            .find("let (map_text_vertices, map_text_indices)")
            .map(|index| draw_features + index)
            .expect("item-frame map text geometry is collected after model translucent features");
        let map_text_draw = source[map_text_collect..]
            .find("self.draw_item_model_geometry(")
            .map(|index| map_text_collect + index)
            .expect("item-frame map text geometry is drawn");
        let block_destroy = source
            .find("label: Some(\"bbb-native-block-destroy-overlay-pass\")")
            .expect("block destroy overlay pass label is used");
        let translucent_target = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");

        assert!(
            copy_particles < entity_pass
                && entity_pass < draw_features
                && draw_features < map_text_collect
                && map_text_collect < map_text_draw
                && map_text_draw < block_destroy
                && block_destroy < translucent_target,
            "vanilla MapRenderer decoration labels are order(1) text submits drawn during renderTranslucentFeatures before crumbling and translucent terrain"
        );
        assert!(
            source[map_text_draw..block_destroy].contains("main_view"),
            "item-frame map text writes the renderer-owned main color target"
        );
    }

    #[test]
    fn item_entity_target_copies_main_depth_and_splits_item_features_from_line_draws() {
        let source = include_str!("render.rs");
        let entity_translucent_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let target = source
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .expect("item-entity target pass label is used");
        let line_target = source
            .find("label: Some(ITEM_ENTITY_LINE_TARGET_PASS_LABEL)")
            .expect("item-entity line target pass label is used");
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
        let block_destroy = source
            .find("label: Some(\"bbb-native-block-destroy-overlay-pass\")")
            .expect("block destroy overlay pass label is used");
        let translucent_target = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent target pass label is used");
        let selection_pipeline = source[line_target..]
            .find("pass.set_pipeline(&self.selection_pipeline)")
            .map(|index| line_target + index)
            .expect("selection line pipeline appends into the item-entity target");
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
                && item_lightmap < block_destroy
                && block_destroy < translucent_target
                && translucent_target < line_target
                && line_target < selection_pipeline
                && selection_pipeline < particle
                && particle < combine,
            "item_entity target copies main depth, draws item features in the vanilla text->item->block phase, then appends lines before particles"
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
            source[target..block_destroy].contains("view: item_entity_view")
                && source[target..block_destroy]
                    .contains("view: &self.item_entity_target.depth.view"),
            "item geometry renders into the item_entity color/depth target before block features"
        );
        assert!(
            source[line_target..particle].contains("load: wgpu::LoadOp::Load")
                && source[line_target..particle].contains("view: item_entity_view")
                && source[line_target..particle]
                    .contains("view: &self.item_entity_target.depth.view"),
            "line geometry appends to the existing item_entity color/depth target before particles"
        );
    }

    #[test]
    fn vanilla_translucent_target_boundaries_are_pinned() {
        let source = include_str!("render.rs");
        let copy_translucent =
            depth_copy_to(source, "texture: &self.translucent_target.depth._texture");
        let copy_item_entity =
            depth_copy_to(source, "texture: &self.item_entity_target.depth._texture");
        let copy_particles = depth_copy_to(source, "texture: &self.particle_target.depth._texture");
        let entity_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let map_text = source
            .find("let (map_text_vertices, map_text_indices)")
            .expect("item-frame map text collection is present");
        let item_entity = source
            .find("label: Some(ITEM_ENTITY_TARGET_PASS_LABEL)")
            .expect("item-entity target pass label is used");
        let block_destroy = source
            .find("label: Some(\"bbb-native-block-destroy-overlay-pass\")")
            .expect("block destroy overlay pass label is used");
        let translucent_terrain = source
            .find("label: Some(TRANSLUCENT_TARGET_PASS_LABEL)")
            .expect("translucent terrain target pass label is used");
        let item_entity_lines = source
            .find("label: Some(ITEM_ENTITY_LINE_TARGET_PASS_LABEL)")
            .expect("item-entity line target pass label is used");
        let particles = source
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .expect("particle target pass label is used");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        assert!(
            copy_translucent < copy_item_entity
                && copy_item_entity < copy_particles
                && copy_particles < entity_features
                && entity_features < map_text
                && map_text < item_entity
                && item_entity < block_destroy
                && block_destroy < translucent_terrain
                && translucent_terrain < item_entity_lines
                && item_entity_lines < particles
                && particles < combine,
            "vanilla LevelRenderer copies target depths, renders translucent features before translucent terrain, and defers translucent particles until after terrain and item_entity lines"
        );
        assert!(
            source[entity_features..translucent_terrain].contains("view: main_view")
                && source[translucent_terrain..item_entity_lines].contains("view: translucent_view")
                && source[item_entity..block_destroy].contains("view: item_entity_view")
                && source[item_entity_lines..particles].contains("view: item_entity_view")
                && source[particles..combine].contains("view: particle_view"),
            "entity features, terrain translucent, itemEntity features/lines, and particles keep distinct renderer-owned targets"
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
                    .contains("entity_model_item_entity_translucent_cull_mesh")
                && source[item_helper..main_helper]
                    .contains("entity_dynamic_player_skin_item_entity_translucent_mesh")
                && source[item_helper..main_helper]
                    .contains("entity_dynamic_player_skin_item_entity_translucent_cull_mesh")
                && source[item_helper..main_helper]
                    .contains("entity_dynamic_player_texture_item_entity_translucent_mesh")
                && source[item_helper..main_helper]
                    .contains("entity_dynamic_player_texture_item_entity_translucent_cull_mesh")
                && source[item_helper..main_helper]
                    .contains("pass.set_pipeline(&self.entity_model_translucent_cull_pipeline)"),
            "static and dynamic item-target translucent entity meshes draw through the item_entity target helper with the cull pipeline where vanilla culls"
        );
        assert!(
            !source[main_helper..tests_mod].contains("entity_model_item_entity_translucent_mesh"),
            "main-target translucent feature helper must not draw ITEM_ENTITY_TARGET render types"
        );
    }

    #[test]
    fn texture_backed_blended_draw_plan_uses_sorted_index_ranges() {
        let source = include_str!("render.rs");
        let item_helper = source
            .find("fn draw_entity_item_entity_target_features")
            .expect("item_entity target feature helper is present");
        let range_helper = source
            .find("fn draw_entity_textured_range")
            .expect("textured range draw helper is present");
        let main_range_helper = source
            .find("fn draw_entity_main_translucent_range")
            .expect("main translucent range draw helper is present");
        let scroll_range_helper = source
            .find("fn draw_entity_scroll_range")
            .expect("scroll range draw helper is present");
        let main_helper = source
            .find("fn draw_entity_translucent_features")
            .expect("main translucent feature helper is present");
        let tests_mod = source
            .find("#[cfg(test)]")
            .expect("render tests module is present");

        assert!(
            source[item_helper..range_helper]
                .contains("self.entity_model_sorted_item_entity_draws"),
            "itemEntity target blended surface draws consume the sorted itemEntity draw plan"
        );
        assert!(
            source[range_helper..main_helper]
                .contains("pass.draw_indexed(draw.index_start..index_end, 0, 0..1)"),
            "sorted blended surface draws must draw only the submission range, not the whole atlas bucket"
        );
        assert!(
            source[main_range_helper..scroll_range_helper]
                .contains("EntityModelTranslucentDrawRange::Scroll(draw)")
                && source[main_range_helper..scroll_range_helper]
                    .contains("EntityModelTranslucentDrawRange::AdditiveScroll(draw)"),
            "main translucent range helper dispatches scroll and additive-scroll draw-plan ranges"
        );
        assert!(
            source[scroll_range_helper..main_helper]
                .contains("pass.draw_indexed(draw.index_start..index_end, 0, 0..1)"),
            "sorted scroll draws must draw only the submission range, not the whole scroll bucket"
        );
        assert!(
            source[main_helper..tests_mod]
                .contains("self.entity_model_sorted_main_translucent_draws"),
            "main translucent blended draws consume the combined textured/scroll draw plan"
        );
        assert!(
            source[main_helper..tests_mod].contains("self.draw_entity_main_translucent_range("),
            "main translucent helper dispatches sorted combined draw-plan ranges"
        );
    }

    #[test]
    fn entity_translucent_emissive_uses_own_pipeline_without_lightmap_bind() {
        let source = include_str!("render.rs");
        let range_helper = source
            .find("fn draw_entity_textured_range")
            .expect("textured range draw helper is present");
        let resources_helper = source
            .find("fn entity_textured_range_resources")
            .expect("textured range resource helper is present");
        let main_helper = source
            .find("fn draw_entity_translucent_features")
            .expect("main translucent feature helper is present");
        let tests_mod = source
            .find("#[cfg(test)]")
            .expect("render tests module is present");

        assert!(
            source[range_helper..resources_helper]
                .contains("EntityModelLayerRenderType::EntityTranslucentEmissive")
                && source[range_helper..resources_helper]
                    .contains("self.entity_model_translucent_emissive_pipeline"),
            "sorted entityTranslucentEmissive ranges use the split vanilla pipeline"
        );
        assert!(
            source[range_helper..resources_helper]
                .contains("if !uses_translucent_emissive_pipeline"),
            "entityTranslucentEmissive skips the LightTexture bind group"
        );
        assert!(
            source[resources_helper..main_helper]
                .contains("self.entity_model_translucent_emissive_mesh"),
            "sorted entityTranslucentEmissive ranges resolve to the dedicated mesh bucket"
        );
        assert!(
            source[main_helper..tests_mod].contains("entity_model_translucent_emissive_mesh")
                && source[main_helper..tests_mod]
                    .contains("self.entity_model_translucent_emissive_pipeline"),
            "fallback unsorted translucent features also draw entityTranslucentEmissive"
        );
    }

    #[test]
    fn armor_render_types_use_dedicated_entity_model_pipelines() {
        let source = include_str!("render.rs");
        let main_pass = source
            .find("label: Some(\"bbb-native-terrain-opaque-group-pass\")")
            .expect("main terrain/entity pass label is used");
        let armor_cutout = source[main_pass..]
            .find("pass.set_pipeline(&self.entity_model_armor_cutout_pipeline)")
            .map(|index| main_pass + index)
            .expect("armorCutoutNoCull pipeline is drawn in the main pass");
        let water_mask = source[main_pass..]
            .find("pass.set_pipeline(&self.entity_model_water_mask_pipeline)")
            .map(|index| main_pass + index)
            .expect("waterMask pipeline is drawn in the main pass");
        let range_helper = source
            .find("fn draw_entity_textured_range")
            .expect("textured range draw helper is present");
        let resources_helper = source
            .find("fn entity_textured_range_resources")
            .expect("textured range resource helper is present");
        let main_helper = source
            .find("fn draw_entity_translucent_features")
            .expect("main translucent feature helper is present");
        let tests_mod = source
            .find("#[cfg(test)]")
            .expect("render tests module is present");

        assert!(
            main_pass < armor_cutout && armor_cutout < water_mask,
            "armorCutoutNoCull uses its dedicated main-pass pipeline before depth-only waterMask"
        );
        assert!(
            source[main_pass..water_mask].contains("entity_model_armor_cutout_mesh")
                && source[main_pass..water_mask]
                    .contains("entity_dynamic_player_texture_armor_cutout_mesh"),
            "static and profile-texture armorCutoutNoCull meshes draw through the armor cutout pipeline"
        );
        assert!(
            source[armor_cutout..water_mask]
                .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])"),
            "armorCutoutNoCull keeps the entity LightTexture bind"
        );
        assert!(
            source[range_helper..resources_helper]
                .contains("EntityModelLayerRenderType::ArmorTranslucent")
                && source[range_helper..resources_helper]
                    .contains("self.entity_model_armor_translucent_pipeline"),
            "sorted armorTranslucent ranges use the split vanilla pipeline"
        );
        assert!(
            source[resources_helper..main_helper]
                .contains("self.entity_model_armor_translucent_mesh"),
            "sorted armorTranslucent ranges resolve to the dedicated mesh bucket"
        );
        assert!(
            source[main_helper..tests_mod].contains("entity_model_armor_translucent_mesh")
                && source[main_helper..tests_mod]
                    .contains("self.entity_model_armor_translucent_pipeline"),
            "fallback unsorted translucent features also draw armorTranslucent"
        );
    }

    #[test]
    fn entity_cutout_z_offset_uses_dedicated_main_pass_pipeline() {
        let source = include_str!("render.rs");
        let main_pass = source
            .find("label: Some(\"bbb-native-terrain-opaque-group-pass\")")
            .expect("main terrain/entity pass label is used");
        let plain_cutout = source[main_pass..]
            .find("pass.set_pipeline(&self.entity_model_textured_pipeline)")
            .map(|index| main_pass + index)
            .expect("plain entityCutout pipeline is drawn in the main pass");
        let z_offset = source[plain_cutout..]
            .find("pass.set_pipeline(&self.entity_model_cutout_z_offset_pipeline)")
            .map(|index| plain_cutout + index)
            .expect("entityCutoutZOffset pipeline is drawn in the main pass");
        let z_offset_block = source[plain_cutout..z_offset]
            .find("&self.entity_model_cutout_z_offset_mesh")
            .map(|index| plain_cutout + index)
            .expect("entityCutoutZOffset dedicated mesh is drawn in the main pass");
        let armor_cutout = source[z_offset..]
            .find("pass.set_pipeline(&self.entity_model_armor_cutout_pipeline)")
            .map(|index| z_offset + index)
            .expect("armorCutoutNoCull pipeline follows z-offset cutout");
        let water_mask = source[armor_cutout..]
            .find("pass.set_pipeline(&self.entity_model_water_mask_pipeline)")
            .map(|index| armor_cutout + index)
            .expect("waterMask pipeline is drawn later in the main pass");

        assert!(
            main_pass < plain_cutout && plain_cutout < z_offset && z_offset < armor_cutout,
            "entityCutoutZOffset draws after plain cutout but before armor/depth-only feature buckets"
        );
        assert!(
            z_offset_block < z_offset
                && source[z_offset_block..armor_cutout]
                    .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])"),
            "static entityCutoutZOffset uses the dedicated mesh and still samples LightTexture"
        );
        assert!(
            source[armor_cutout..water_mask]
                .contains("entity_dynamic_player_skin_cutout_z_offset_mesh")
                && source[armor_cutout..water_mask]
                    .contains("entity_dynamic_player_texture_cutout_z_offset_mesh"),
            "dynamic atlas z-offset buckets also route through the main-pass cutout-z-offset pipeline"
        );
    }

    #[test]
    fn entity_glint_draws_in_main_pass_without_lightmap_bind() {
        let source = include_str!("render.rs");
        let main_pass = source
            .find("label: Some(\"bbb-native-terrain-opaque-group-pass\")")
            .expect("main terrain/entity pass label is used");
        let armor_glint = source[main_pass..]
            .find("self.entity_model_armor_entity_glint_pipeline")
            .map(|index| main_pass + index)
            .expect("armorEntityGlint pipeline is drawn");
        let entity_glint = source[armor_glint..]
            .find("self.entity_model_entity_glint_pipeline")
            .map(|index| armor_glint + index)
            .expect("entityGlint pipeline is drawn");
        let depth_copy = depth_copy_to(source, "texture: &self.translucent_target.depth._texture");

        assert!(
            main_pass < armor_glint && armor_glint < entity_glint && entity_glint < depth_copy,
            "entity glint render types are main-target entity feature draws before target depth copies"
        );
        assert!(
            !source[armor_glint..entity_glint]
                .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
                && !source[entity_glint..depth_copy]
                    .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])"),
            "vanilla glint pipeline does not bind LightTexture"
        );
    }

    #[test]
    fn entity_water_mask_draws_depth_only_before_depth_copies() {
        let source = include_str!("render.rs");
        let main_pass = source
            .find("label: Some(\"bbb-native-terrain-opaque-group-pass\")")
            .expect("main terrain/entity pass label is used");
        let water_mask = source[main_pass..]
            .find("pass.set_pipeline(&self.entity_model_water_mask_pipeline)")
            .map(|index| main_pass + index)
            .expect("waterMask pipeline is drawn");
        let water_mask_draw = source[water_mask..]
            .find("pass.draw_indexed")
            .map(|index| water_mask + index)
            .expect("waterMask draw follows pipeline");
        let armor_glint = source[water_mask_draw..]
            .find("self.entity_model_armor_entity_glint_pipeline")
            .map(|index| water_mask_draw + index)
            .expect("armorEntityGlint pipeline follows waterMask");
        let depth_copy = depth_copy_to(source, "texture: &self.translucent_target.depth._texture");

        assert!(
            main_pass < water_mask
                && water_mask < water_mask_draw
                && water_mask_draw < armor_glint
                && armor_glint < depth_copy,
            "waterMask is a main-target depth write before glint and target depth copies"
        );
        assert!(
            source[water_mask..water_mask_draw]
                .contains("pass.set_bind_group(0, &self.terrain_bind_group, &[])"),
            "waterMask binds only the camera/terrain group, not the entity texture atlas"
        );
        assert!(
            !source[water_mask..water_mask_draw]
                .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])"),
            "vanilla waterMask does not bind LightTexture"
        );
    }

    #[test]
    fn particle_target_copies_main_depth_and_clears_before_combine() {
        let source = include_str!("render.rs");
        let particle_fn = source
            .find("fn particle_target_pass")
            .expect("particle target pass function is present");
        let particle_fn_end = source[particle_fn..]
            .find("fn entity_outline_post_chain")
            .map(|index| particle_fn + index)
            .expect("particle target pass ends before outline post-chain");
        let entity_translucent_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let target = source[particle_fn..]
            .find("label: Some(PARTICLE_TARGET_PASS_LABEL)")
            .map(|index| particle_fn + index)
            .expect("particle target pass label is used");
        let copy_depth = depth_copy_to(source, "texture: &self.particle_target.depth._texture");
        let experience_orb_upload = source[particle_fn..target]
            .find("upload_experience_orb_pickup_particle_textured_mesh(")
            .map(|index| particle_fn + index)
            .expect("experience orb pickup particle mesh is uploaded before the render pass");
        let elder_upload = source[particle_fn..target]
            .find("upload_elder_guardian_particle_textured_mesh(")
            .map(|index| particle_fn + index)
            .expect("elder guardian particle model mesh is uploaded before the render pass");
        // Opaque particles draw into the main color+depth target during
        // `opaque_particle_main_pass`, before the main depth is copied into the feature
        // targets; only the translucent half remains in the particles target.
        let opaque_main_fn = source
            .find("fn opaque_particle_main_pass")
            .expect("opaque particle main pass function is present");
        let opaque_main_label = source[opaque_main_fn..]
            .find("label: Some(OPAQUE_PARTICLE_MAIN_PASS_LABEL)")
            .map(|index| opaque_main_fn + index)
            .expect("opaque particle main pass label is used");
        let opaque_particle_pipeline = source[opaque_main_label..]
            .find("pass.set_pipeline(&self.opaque_particle_pipeline)")
            .map(|index| opaque_main_label + index)
            .expect("opaque particle pipeline is drawn into main");
        let opaque_particle_draw = source[opaque_particle_pipeline..]
            .find("&particle_vertex_batches.opaque")
            .map(|index| opaque_particle_pipeline + index)
            .expect("opaque particle vertex batch is drawn into main");
        let translucent_particle_pipeline = source[target..]
            .find("pass.set_pipeline(&self.translucent_particle_pipeline)")
            .map(|index| target + index)
            .expect("translucent particle pipeline is drawn into the target");
        let translucent_particle_draw = source[translucent_particle_pipeline..]
            .find("&particle_vertex_batches.translucent")
            .map(|index| translucent_particle_pipeline + index)
            .expect("translucent particle vertex batch is drawn into the target");
        let experience_orb_particle_pipeline = source[translucent_particle_draw..particle_fn_end]
            .find("pass.set_pipeline(&self.entity_model_translucent_cull_pipeline)")
            .map(|index| translucent_particle_draw + index)
            .expect(
                "experience orb pickup particles draw through the entity translucent-cull pipeline",
            );
        let experience_orb_particle_draw = source
            [experience_orb_particle_pipeline..particle_fn_end]
            .find("pass.draw_indexed(0..index_count, 0, 0..1)")
            .map(|index| experience_orb_particle_pipeline + index)
            .expect("experience orb pickup particle draw follows its pipeline");
        let elder_particle_pipeline = source[experience_orb_particle_draw..particle_fn_end]
            .find("pass.set_pipeline(&self.entity_model_translucent_pipeline)")
            .map(|index| experience_orb_particle_draw + index)
            .expect("elder guardian particles draw through the entity translucent pipeline");
        let elder_particle_draw = source[elder_particle_pipeline..particle_fn_end]
            .find("pass.draw_indexed(0..index_count, 0, 0..1)")
            .map(|index| elder_particle_pipeline + index)
            .expect("elder guardian particle model draw follows its pipeline");
        let draw_helper = source
            .find("fn draw_particle_vertex_batch")
            .expect("particle draw helper is present");
        let helper_atlas = source[draw_helper..]
            .find("pass.set_bind_group(0, bind_group, &[])")
            .map(|index| draw_helper + index)
            .expect("particle atlas bind group is bound before draw");
        let helper_lightmap = source[helper_atlas..]
            .find("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
            .map(|index| helper_atlas + index)
            .expect("particle lightmap bind group is bound before draw");
        let helper_draw = source[helper_lightmap..]
            .find("pass.draw(draw.vertex_start..draw.vertex_end(), 0..1)")
            .map(|index| helper_lightmap + index)
            .expect("particle draw range is issued after bind groups");
        let bind_helper = source
            .find("fn particle_texture_atlas_bind_group")
            .expect("particle texture atlas bind helper is present");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        assert!(
            opaque_main_label < opaque_particle_pipeline
                && opaque_particle_pipeline < opaque_particle_draw
                && opaque_particle_draw < copy_depth
                && copy_depth < entity_translucent_features
                && entity_translucent_features < target
                && experience_orb_upload < target
                && experience_orb_upload < elder_upload
                && elder_upload < target
                && target < translucent_particle_pipeline
                && translucent_particle_pipeline < translucent_particle_draw
                && translucent_particle_draw < experience_orb_particle_pipeline
                && experience_orb_particle_pipeline < experience_orb_particle_draw
                && experience_orb_particle_draw < elder_particle_pipeline
                && elder_particle_pipeline < elder_particle_draw
                && elder_particle_draw < combine,
            "opaque particles draw into main before the main depth copy; the particles target then copies main depth, clears transparent, draws translucent particles, and transparency combine consumes it"
        );
        assert!(
            source[opaque_main_label..opaque_particle_draw].contains("view: main_view")
                && source[opaque_main_label..opaque_particle_draw]
                    .contains("view: &self.depth.view")
                && source[opaque_main_label..opaque_particle_draw]
                    .contains("load: wgpu::LoadOp::Load"),
            "opaque particles draw into the main color+depth target with a load so they never clear the main pass output, matching vanilla ParticleFeatureRenderer opaque draws"
        );
        assert!(
            source[experience_orb_upload..target]
                .contains("frame_experience_orb_pickup_particle_vertices")
                && source[experience_orb_upload..target]
                    .contains("frame_experience_orb_pickup_particle_indices")
                && source[experience_orb_upload..target].contains("&atlas.layout"),
            "experience orb pickup particles upload into renderer-owned frame buffers with the entity atlas layout"
        );
        assert!(
            source[elder_upload..target].contains("frame_elder_guardian_particle_vertices")
                && source[elder_upload..target].contains("frame_elder_guardian_particle_indices")
                && source[elder_upload..target].contains("&atlas.layout"),
            "elder guardian particles upload into renderer-owned frame buffers with the entity atlas layout"
        );
        assert!(
            source[experience_orb_particle_pipeline..experience_orb_particle_draw]
                .contains("pass.set_bind_group(0, &atlas.bind_group, &[])")
                && source[experience_orb_particle_pipeline..experience_orb_particle_draw]
                    .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
                && source[experience_orb_particle_pipeline..experience_orb_particle_draw]
                    .contains("self.frame_experience_orb_pickup_particle_vertices")
                && source[experience_orb_particle_pipeline..experience_orb_particle_draw]
                    .contains("self.frame_experience_orb_pickup_particle_indices"),
            "experience orb pickup particles bind the entity atlas, lightmap, and persistent frame buffers before drawing"
        );
        assert!(
            source[elder_particle_pipeline..elder_particle_draw]
                .contains("pass.set_bind_group(0, &atlas.bind_group, &[])")
                && source[elder_particle_pipeline..elder_particle_draw]
                    .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
                && source[elder_particle_pipeline..elder_particle_draw]
                    .contains("self.frame_elder_guardian_particle_vertices")
                && source[elder_particle_pipeline..elder_particle_draw]
                    .contains("self.frame_elder_guardian_particle_indices"),
            "elder guardian particles bind the entity atlas, lightmap, and persistent frame buffers before drawing"
        );
        // Item-pickup carried arrow/trident models: uploaded before the pass and
        // drawn inside the ITEM_PICKUP group order, after the orb-icon draw and
        // before the elder-guardian group, through the same translucent-cull
        // entity pipeline as the orb billboard.
        let projectile_pickup_upload = source[particle_fn..target]
            .find("upload_projectile_pickup_particle_textured_mesh(")
            .map(|index| particle_fn + index)
            .expect("projectile pickup particle mesh is uploaded before the render pass");
        let projectile_pickup_pipeline = source[experience_orb_particle_draw..particle_fn_end]
            .find("pass.set_pipeline(&self.entity_model_translucent_cull_pipeline)")
            .map(|index| experience_orb_particle_draw + index)
            .expect(
                "projectile pickup particles draw through the entity translucent-cull pipeline",
            );
        assert!(
            projectile_pickup_upload < target
                && experience_orb_particle_draw < projectile_pickup_pipeline
                && projectile_pickup_pipeline < elder_particle_pipeline,
            "projectile pickup carried models draw between the orb-icon and elder-guardian particle draws"
        );
        assert!(
            source[projectile_pickup_pipeline..elder_particle_pipeline]
                .contains("pass.set_bind_group(0, &atlas.bind_group, &[])")
                && source[projectile_pickup_pipeline..elder_particle_pipeline]
                    .contains("pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[])")
                && source[projectile_pickup_pipeline..elder_particle_pipeline]
                    .contains("self.frame_projectile_pickup_particle_vertices")
                && source[projectile_pickup_pipeline..elder_particle_pipeline]
                    .contains("self.frame_projectile_pickup_particle_indices"),
            "projectile pickup particles bind the entity atlas, lightmap, and persistent frame buffers before drawing"
        );
        assert!(
            !source[particle_fn..particle_fn_end].contains("create_buffer_init"),
            "particle target per-frame uploads must use FrameDataBuffer rather than create_buffer_init"
        );
        assert!(
            helper_atlas < helper_lightmap && helper_lightmap < helper_draw,
            "particle draw ranges bind their selected atlas and the renderer-owned LightTexture before drawing"
        );
        assert!(
            source[bind_helper..].contains("ParticleTextureAtlasKind::Particles")
                && source[bind_helper..].contains("self.particle_atlas")
                && source[bind_helper..].contains(
                    "ParticleTextureAtlasKind::Terrain => Some(&self.terrain_bind_group)"
                )
                && source[bind_helper..].contains("ParticleTextureAtlasKind::Items")
                && source[bind_helper..].contains("self.item_entity_atlas"),
            "particle draw ranges can bind vanilla particles, blocks/terrain, and item atlases"
        );
        assert!(
            source[copy_depth..entity_translucent_features]
                .contains("texture: &self.depth._texture")
                && source[copy_depth..entity_translucent_features]
                    .contains("texture: &self.particle_target.depth._texture"),
            "particle target depth is copied from the renderer-owned main depth texture"
        );
        assert!(
            source[target..translucent_particle_pipeline]
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
    fn world_border_draws_into_weather_target_after_rain_snow_and_before_combine() {
        let source = include_str!("render.rs");
        let target = source
            .find("label: Some(WEATHER_TARGET_PASS_LABEL)")
            .expect("weather target pass label is used");
        let weather = source[target..]
            .find("pass.set_pipeline(&self.weather_pipeline)")
            .map(|index| target + index)
            .expect("weather target pass draws rain/snow pipeline");
        let world_border = source[target..]
            .find("pass.set_pipeline(&self.world_border_pipeline)")
            .map(|index| target + index)
            .expect("weather target pass draws world border pipeline");
        let combine = source
            .find("label: Some(TRANSPARENCY_COMBINE_PASS_LABEL)")
            .expect("transparency combine pass label is used");

        // Vanilla LevelRenderer.addWeatherPass runs worldBorderRenderer.render
        // after weatherEffectRenderer.render (LevelRenderer.java:751-758), and
        // WorldBorderRenderer.render targets the weather color+depth when the
        // weather target exists (WorldBorderRenderer.java:143-150).
        assert!(
            target < weather && weather < world_border && world_border < combine,
            "world border draws into the weather target after rain/snow and before transparency combine"
        );
        assert!(
            source[world_border..combine]
                .contains("pass.set_bind_group(0, &texture.bind_group, &[])"),
            "world border binds the forcefield texture bind group (camera + Sampler0)"
        );
        assert!(
            source[weather..world_border].contains("stats.weather_draw_calls += 1;")
                && source[world_border..combine].contains("stats.weather_draw_calls += 1;"),
            "world border draw is tallied with the weather target pass draws"
        );
    }

    #[test]
    fn transparency_combine_writes_internal_final_then_blits_surface_before_hud_and_screenshot_readback(
    ) {
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
        let blit = source
            .find("label: Some(TRANSPARENCY_BLIT_PASS_LABEL)")
            .expect("transparency blit pass label is used");
        let hud_pass = source
            .find("label: Some(\"bbb-native-hud-pass\")")
            .expect("hud pass label is used");
        let hud_item_pass = source
            .find("label: Some(\"bbb-native-hud-item-pass\")")
            .expect("hud item pass label is used");
        let hud_overlay_pass = source
            .find("label: Some(\"bbb-native-hud-overlay-pass\")")
            .expect("hud overlay pass label is used");
        let screenshot_copy = source
            .find("prepare_screenshot_copy")
            .expect("screenshot copy still reads the presented frame");

        assert!(
            main_view < terrain_pass && terrain_pass < combine,
            "content passes draw to the renderer-owned main target before transparency combine"
        );
        assert!(
            !source[..combine].contains("view: &surface_view"),
            "surface view is not a render target before the transparency chain starts"
        );
        assert!(
            source[terrain_pass..combine].contains("view: main_view"),
            "main content passes use the renderer-owned main target"
        );
        assert!(
            source[combine..blit].contains("view: &self.transparency_final_target.view"),
            "transparency combine writes the vanilla-shaped internal final target"
        );
        assert!(
            source[combine..blit].contains(
                "pass.set_bind_group(0, &self.transparency_combine_bind_group.bind_group, &[])"
            ),
            "transparency combine samples the renderer-owned target bundle"
        );
        assert!(
            source[combine..blit]
                .contains("pass.set_pipeline(&self.transparency_combine_pipeline)"),
            "combine uses the transparency shader before the blit pass"
        );
        assert!(
            !source[..blit].contains("view: &surface_view"),
            "surface view is not a render target until the vanilla transparency final blit"
        );
        assert!(
            source[blit..hud_pass].contains("view: &surface_view"),
            "transparency blit writes the swapchain surface before HUD rendering"
        );
        assert!(
            source[blit..hud_pass].contains(
                "pass.set_bind_group(0, &self.transparency_final_target.bind_group, &[])"
            ),
            "transparency blit samples the internal final target"
        );
        assert!(
            combine < blit
                && blit < hud_pass
                && hud_pass < hud_item_pass
                && hud_item_pass < hud_overlay_pass
                && hud_overlay_pass < screenshot_copy,
            "HUD, GUI item, and HUD overlay passes draw on the surface after transparency final blit"
        );
        assert!(
            source[hud_pass..screenshot_copy].contains("view: &surface_view"),
            "post-blit HUD passes target the swapchain surface"
        );
    }

    #[test]
    fn hud_gui_3d_items_draw_between_hud_base_and_overlay_decorations() {
        let source = include_str!("render.rs");
        let pre_commands = source
            .find("&hud_draws.commands[..hud_draws.post_gui_item_start]")
            .expect("pre-GUI-item HUD command slice is drawn first");
        let hud_pass = source[pre_commands..]
            .find("label: Some(\"bbb-native-hud-pass\")")
            .map(|index| pre_commands + index)
            .expect("base HUD pass label is used");
        let gui_items = source[hud_pass..]
            .find("let gui_item_meshes = self.collect_hud_block_item_mesh()")
            .map(|index| hud_pass + index)
            .expect("GUI 3D item mesh is collected after base HUD");
        let hud_item_pass = source[gui_items..]
            .find("label: Some(\"bbb-native-hud-item-pass\")")
            .map(|index| gui_items + index)
            .expect("GUI 3D item pass label is used");
        let post_commands = source[hud_item_pass..]
            .find("&hud_draws.commands[hud_draws.post_gui_item_start..]")
            .map(|index| hud_item_pass + index)
            .expect("post-GUI-item HUD command slice is drawn last");
        let hud_overlay_pass = source[post_commands..]
            .find("label: Some(\"bbb-native-hud-overlay-pass\")")
            .map(|index| post_commands + index)
            .expect("HUD overlay pass label is used");

        assert!(
            pre_commands < hud_pass
                && hud_pass < gui_items
                && gui_items < hud_item_pass
                && hud_item_pass < post_commands
                && post_commands < hud_overlay_pass,
            "GUI 3D item models draw after base HUD/slot backgrounds and before decorations/front overlays"
        );
    }

    #[test]
    fn hud_gui_3d_items_draw_translucent_base_and_glint_in_gui_item_pass() {
        let source = include_str!("render.rs");
        let gui_items = source
            .find("let gui_item_meshes = self.collect_hud_block_item_mesh()")
            .expect("GUI 3D item mesh is collected");
        let translucent_buffers = source[gui_items..]
            .find("&gui_item_meshes.translucent.vertices")
            .map(|index| gui_items + index)
            .expect("GUI 3D item translucent buffers are created");
        let glint_translucent_buffers = source[translucent_buffers..]
            .find("&gui_item_meshes.glint_translucent.vertices")
            .map(|index| translucent_buffers + index)
            .expect("GUI 3D item glintTranslucent buffers are created");
        let hud_item_pass = source[glint_translucent_buffers..]
            .find("label: Some(\"bbb-native-hud-item-pass\")")
            .map(|index| glint_translucent_buffers + index)
            .expect("GUI 3D item pass label is used");
        let solid_draw = source[hud_item_pass..]
            .find("&self.item_model_pipeline")
            .map(|index| hud_item_pass + index)
            .expect("solid GUI 3D item base draws first");
        let solid_glint = source[solid_draw..]
            .find("self.draw_item_model_glint_frame_buffers(")
            .map(|index| solid_draw + index)
            .expect("solid GUI 3D item glint follows solid base");
        let translucent_draw = source[solid_glint..]
            .find("&self.item_model_translucent_pipeline")
            .map(|index| solid_glint + index)
            .expect("translucent GUI 3D item base follows solid glint");
        let translucent_glint = source[translucent_draw..]
            .find("self.draw_item_model_glint_frame_buffers(")
            .map(|index| translucent_draw + index)
            .expect("GUI 3D item glintTranslucent follows translucent base");
        let post_commands = source[translucent_glint..]
            .find("&hud_draws.commands[hud_draws.post_gui_item_start..]")
            .map(|index| translucent_glint + index)
            .expect("post-GUI HUD commands follow GUI item pass");

        assert!(
            hud_item_pass < solid_draw
                && solid_draw < solid_glint
                && solid_glint < translucent_draw
                && translucent_draw < translucent_glint
                && translucent_glint < post_commands,
            "GUI 3D item translucent base and glintTranslucent stay inside the GUI item pass"
        );
    }

    #[test]
    fn cloud_presentation_binds_cloud_offset_uniform() {
        let source = include_str!("render.rs");
        let clouds = source
            .find("pass.set_pipeline(cloud_pipeline)")
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
    fn entity_outline_target_stays_in_main_pass_and_post_chain_runs_before_clouds() {
        // Vanilla LevelRenderer writes the entity_outline target from addMainPass, then adds
        // the entity_outline post chain immediately after addMainPass and before clouds,
        // weather, and transparency.
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
        let entity_translucent_features = source
            .find("label: Some(ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL)")
            .expect("entity translucent feature pass label is used");
        let block_destroy = source
            .find("label: Some(\"bbb-native-block-destroy-overlay-pass\")")
            .expect("block destroy overlay pass label is used");
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
            entity_translucent_features < block_destroy
                && block_destroy < target
                && target < translucent
                && translucent < particle
                && particle < sobel
                && composite < clouds
                && clouds < weather
                && weather < combine,
            "outline target write stays in the main pass while the post-chain runs before vanilla cloud/weather/transparency passes"
        );
    }
}
