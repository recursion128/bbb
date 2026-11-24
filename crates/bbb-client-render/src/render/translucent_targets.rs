//! Route-selected translucent world draws and their frame-local uploads.

use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct OmittedTransparencyTargets {
    pub(super) item_entity: bool,
    pub(super) particles: bool,
    pub(super) weather: bool,
    pub(super) clouds: bool,
}

impl OmittedTransparencyTargets {
    pub(super) fn mask(self) -> u32 {
        u32::from(self.item_entity)
            | (u32::from(self.particles) << 1)
            | (u32::from(self.weather) << 2)
            | (u32::from(self.clouds) << 3)
    }
}

pub(super) struct PreparedWorldTargets {
    pub(super) opaque_particles: bool,
    translucent_particles: bool,
    item_entities: u32,
    fishing_lines: u32,
    opaque_debug_lines: u32,
    translucent_debug_lines: u32,
    experience_orbs: Option<u32>,
    projectiles: Option<u32>,
    elder_guardians: Option<u32>,
    weather: Option<(std::ops::Range<u32>, std::ops::Range<u32>)>,
    lightning: Option<u32>,
    border: Option<u32>,
}

impl PreparedWorldTargets {
    pub(super) fn weather_has_depth_work(&self) -> bool {
        self.weather.is_some() || self.lightning.is_some() || self.border.is_some()
    }
}

impl Renderer {
    pub(super) fn prepare_world_targets(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        particles: &ParticleBatches,
    ) -> PreparedWorldTargets {
        let instanced = self.uses_instanced_particles();
        let opaque_particles = particles.opaque.upload_staged(
            &mut self.frame_opaque_particle_vertices,
            &self.device,
            encoder,
            self.frame_uploads.belt(),
            instanced,
        );
        let translucent_particles = particles.translucent.upload_staged(
            &mut self.frame_translucent_particle_vertices,
            &self.device,
            encoder,
            self.frame_uploads.belt(),
            instanced,
        );
        let vertices = self.collect_item_entity_vertices();
        let item_entities = if self.pack.item_entity_atlas.is_some()
            && self.frame_item_entity_vertices.upload_staged(
                &self.device,
                encoder,
                self.frame_uploads.belt(),
                bytemuck::cast_slice(&vertices),
            ) {
            vertices.len() as u32
        } else {
            0
        };
        let width = (self.config.width as f32 / 1920.0 * 2.5).max(2.5);
        let lines = self.camera_pose.map_or_else(Vec::new, |_| {
            self.entity_model_instances
                .iter()
                .filter_map(|instance| fishing_hook_line_segments(instance, width))
                .flatten()
                .collect::<Vec<_>>()
        });
        let vertices = selection_lines_vertices(&lines);
        let fishing_lines = if self.frame_entity_model_line_vertices.upload_staged(
            &self.device,
            encoder,
            self.frame_uploads.belt(),
            bytemuck::cast_slice(&vertices),
        ) {
            vertices.len() as u32
        } else {
            0
        };
        let debug = self.camera_pose.map_or_else(Default::default, |camera| {
            debug_line_vertex_batches(&self.debug_lines, camera)
        });
        let opaque_debug_lines = if self.frame_debug_line_opaque_vertices.upload_staged(
            &self.device,
            encoder,
            self.frame_uploads.belt(),
            bytemuck::cast_slice(&debug.opaque),
        ) {
            debug.opaque.len() as u32
        } else {
            0
        };
        let translucent_debug_lines = if self.frame_debug_line_translucent_vertices.upload_staged(
            &self.device,
            encoder,
            self.frame_uploads.belt(),
            bytemuck::cast_slice(&debug.translucent),
        ) {
            debug.translucent.len() as u32
        } else {
            0
        };
        let orbs = std::mem::take(&mut self.particle_frame.experience_orbs);
        let experience_orbs = self
            .pack
            .entity_model_texture_atlas
            .as_ref()
            .and_then(|atlas| {
                upload_experience_orb_pickup_particle_textured_mesh(
                    &self.device,
                    encoder,
                    self.frame_uploads.belt(),
                    &mut self.frame_experience_orb_pickup_particle_vertices,
                    &mut self.frame_experience_orb_pickup_particle_indices,
                    &orbs,
                    &atlas.layout,
                )
            });
        let projectiles = std::mem::take(&mut self.particle_frame.projectiles);
        let projectiles = self
            .pack
            .entity_model_texture_atlas
            .as_ref()
            .and_then(|atlas| {
                upload_projectile_pickup_particle_textured_mesh(
                    &self.device,
                    encoder,
                    self.frame_uploads.belt(),
                    &mut self.frame_projectile_pickup_particle_vertices,
                    &mut self.frame_projectile_pickup_particle_indices,
                    &projectiles,
                    &atlas.layout,
                )
            });
        let guardians = std::mem::take(&mut self.particle_frame.elder_guardians);
        let elder_guardians = self
            .pack
            .entity_model_texture_atlas
            .as_ref()
            .and_then(|atlas| {
                upload_elder_guardian_particle_textured_mesh(
                    &self.device,
                    encoder,
                    self.frame_uploads.belt(),
                    &mut self.frame_elder_guardian_particle_vertices,
                    &mut self.frame_elder_guardian_particle_indices,
                    &guardians,
                    &atlas.layout,
                )
            });
        let weather = build_weather_mesh(&self.weather_render_state)
            .filter(|mesh| {
                (!mesh.rain_indices.is_empty() && self.pack.weather_rain_texture.is_some())
                    || (!mesh.snow_indices.is_empty() && self.pack.weather_snow_texture.is_some())
            })
            .and_then(|mesh| {
                let uploaded = self.frame_weather_vertices.upload_staged(
                    &self.device,
                    encoder,
                    self.frame_uploads.belt(),
                    bytemuck::cast_slice(&mesh.vertices),
                ) && self.frame_weather_indices.upload_staged(
                    &self.device,
                    encoder,
                    self.frame_uploads.belt(),
                    bytemuck::cast_slice(&mesh.indices),
                );
                uploaded.then_some((mesh.rain_indices, mesh.snow_indices))
            });
        let lightning = build_lightning_mesh(&self.weather_render_state).and_then(|mesh| {
            let uploaded = self.frame_lightning_vertices.upload_staged(
                &self.device,
                encoder,
                self.frame_uploads.belt(),
                bytemuck::cast_slice(&mesh.vertices),
            ) && self.frame_lightning_indices.upload_staged(
                &self.device,
                encoder,
                self.frame_uploads.belt(),
                bytemuck::cast_slice(&mesh.indices),
            );
            uploaded.then_some(mesh.indices.len() as u32)
        });
        let border = build_world_border_mesh(&self.world_border_render_state).and_then(|mesh| {
            let uploaded = self.frame_world_border_vertices.upload_staged(
                &self.device,
                encoder,
                self.frame_uploads.belt(),
                bytemuck::cast_slice(&mesh.vertices),
            ) && self.frame_world_border_indices.upload_staged(
                &self.device,
                encoder,
                self.frame_uploads.belt(),
                bytemuck::cast_slice(&mesh.indices),
            );
            uploaded.then_some(mesh.indices.len() as u32)
        });
        PreparedWorldTargets {
            opaque_particles,
            translucent_particles,
            item_entities,
            fishing_lines,
            opaque_debug_lines,
            translucent_debug_lines,
            experience_orbs,
            projectiles,
            elder_guardians,
            weather,
            lightning,
            border,
        }
    }

    pub(super) fn draw_opaque_particles<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
        particles: &'a ParticleBatches,
        prepared: &PreparedWorldTargets,
    ) {
        if prepared.opaque_particles {
            pass.set_pipeline(&self.pipelines.opaque_particle);
            stats.pipeline_switches += 1;
            self.draw_particle_batch(
                pass,
                self.frame_opaque_particle_vertices
                    .buffer()
                    .expect("prepared opaque particles"),
                &particles.opaque,
                stats,
            );
        }
    }

    pub(super) fn has_item_entity_lines(&self, prepared: &PreparedWorldTargets) -> bool {
        self.block_selection
            .has_phase(BlockSelectionPhase::AfterTranslucent)
            || self.chunk_border_outline.has_geometry()
            || self.entity_scene_outline.has_geometry()
            || prepared.fishing_lines != 0
            || prepared.opaque_debug_lines != 0
            || prepared.translucent_debug_lines != 0
    }

    pub(super) fn draw_clouds(&self, pass: &mut wgpu::RenderPass<'_>, stats: &mut FrameDrawStats) {
        let clouds = self.pack.clouds.as_ref().expect("cloud draw geometry");
        let pipeline = match self.cloud_shape {
            CloudShape::Flat => &self.pipelines.cloud_flat,
            CloudShape::Fancy => &self.pipelines.cloud_fancy,
        };
        pass.set_pipeline(pipeline);
        stats.pipeline_switches += 1;
        pass.set_bind_group(0, &self.pack.terrain_bind_group, &[]);
        pass.set_bind_group(1, &self.cloud_bind_group, &[]);
        pass.set_vertex_buffer(0, clouds.face_buffer.slice(..));
        pass.set_index_buffer(
            self.cloud_quad_indices.as_ref().unwrap().slice(..),
            wgpu::IndexFormat::Uint32,
        );
        pass.draw_indexed(0..6, 0, 0..clouds.face_count);
        stats.sky_draw_calls += 1;
    }
    fn has_item_entity_target_models(&self) -> bool {
        use crate::item_models::{ItemModelDrawPhase, ItemModelDrawTarget};

        !self.pack.entity_meshes.sorted_item_entity_draws.is_empty()
            || self.pack.entity_meshes.has_any(&[
                EntityMeshLane::ItemEntityTranslucent,
                EntityMeshLane::ItemEntityTranslucentCull,
                EntityMeshLane::SkinItemEntityTranslucent,
                EntityMeshLane::SkinItemEntityTranslucentCull,
                EntityMeshLane::PlayerTextureItemEntityTranslucent,
                EntityMeshLane::PlayerTextureItemEntityTranslucentCull,
            ])
            || self
                .pack
                .item_model_residency
                .draws_for_target(ItemModelDrawTarget::World)
                .any(|draw| {
                    matches!(
                        draw.phase,
                        ItemModelDrawPhase::Translucent
                            | ItemModelDrawPhase::TranslucentNoCull
                            | ItemModelDrawPhase::GlintTranslucent
                    )
                })
            || self
                .pack
                .item_model_residency
                .draws_for_target(ItemModelDrawTarget::BlockFeature)
                .any(|draw| draw.phase == ItemModelDrawPhase::Translucent)
    }

    pub(super) fn item_entity_target_has_depth_work(&self) -> bool {
        !self.item_entity_billboards.is_empty()
            || self.block_selection.is_active()
            || self.chunk_border_outline.has_geometry()
            || self.entity_scene_outline.has_geometry()
            || (self.camera_pose.is_some()
                && (!self.entity_model_instances.is_empty() || !self.debug_lines.is_empty()))
            || self.has_item_entity_target_models()
    }

    pub(super) fn omitted_transparency_targets(
        &self,
        prepared: &PreparedWorldTargets,
    ) -> OmittedTransparencyTargets {
        let item_entity = prepared.item_entities != 0
            || self.block_selection.is_active()
            || self.has_item_entity_lines(prepared)
            || self.has_item_entity_target_models();
        let particles = prepared.translucent_particles
            || prepared.elder_guardians.is_some()
            || prepared.experience_orbs.is_some()
            || prepared.projectiles.is_some()
            || self.has_resident_particle_items();
        OmittedTransparencyTargets {
            item_entity: !item_entity,
            particles: !particles,
            weather: !prepared.weather_has_depth_work(),
            clouds: !self.has_cloud_draws(),
        }
    }

    pub(super) fn particle_target_has_depth_work(
        &self,
        particle_batches: &ParticleBatches,
    ) -> bool {
        !particle_batches.translucent.quads.is_empty()
            || self.particle_frame.live
            || self.has_resident_particle_items()
    }

    fn has_resident_particle_items(&self) -> bool {
        self.pack
            .item_model_residency
            .has_draws_for_target(crate::item_models::ItemModelDrawTarget::Particle)
    }

    pub(super) fn draw_item_entity_target<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
        prepared: &PreparedWorldTargets,
    ) {
        self.draw_entity_item_entity_target_features(
            pass,
            &mut stats.pipeline_switches,
            &mut stats.entity_model_draw_calls,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::World,
            &[crate::item_models::ItemModelDrawPhase::Translucent],
            Some(&[crate::item_models::ItemModelAtlasKind::Blocks]),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::World,
            &[crate::item_models::ItemModelDrawPhase::Translucent],
            Some(&[crate::item_models::ItemModelAtlasKind::Items]),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::World,
            &[
                crate::item_models::ItemModelDrawPhase::Translucent,
                crate::item_models::ItemModelDrawPhase::TranslucentNoCull,
            ],
            Some(RESIDENT_SPECIAL_ITEM_ATLASES),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::World,
            &[crate::item_models::ItemModelDrawPhase::GlintTranslucent],
            None,
            stats,
        );
        if let (Some(atlas), true) = (&self.pack.item_entity_atlas, prepared.item_entities != 0) {
            let vertex_buffer = self.frame_item_entity_vertices.buffer().expect("uploaded");
            pass.set_pipeline(&self.pipelines.item_entity);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..prepared.item_entities, 0..1);
            stats.item_entity_draw_calls += 1;
        }
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::BlockFeature,
            &[crate::item_models::ItemModelDrawPhase::Translucent],
            None,
            stats,
        );
    }

    pub(super) fn draw_selection_before_translucent(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut FrameDrawStats,
    ) {
        if !self
            .block_selection
            .has_phase(BlockSelectionPhase::BeforeTranslucent)
        {
            return;
        }
        pass.set_bind_group(0, &self.pack.terrain_bind_group, &[]);
        if let Some((secondary, count)) = self.block_selection.secondary_vertices() {
            pass.set_pipeline(&self.pipelines.translucent_debug_line);
            pass.set_vertex_buffer(0, secondary.slice(..));
            pass.draw(0..count, 0..1);
            stats.pipeline_switches += 1;
        }
        if let Some((vertices, count)) = self.block_selection.vertices() {
            pass.set_pipeline(&self.pipelines.selection);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.draw(0..count, 0..1);
            stats.pipeline_switches += 1;
            stats.selection_draw_calls += 1;
        }
    }

    pub(super) fn draw_translucent_terrain(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut FrameDrawStats,
    ) {
        self.draw_terrain_layer(pass, TerrainLayer::Translucent, stats);
    }

    pub(super) fn draw_item_entity_lines<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
        prepared: &PreparedWorldTargets,
    ) {
        let block_selection_after_translucent = self
            .block_selection
            .has_phase(BlockSelectionPhase::AfterTranslucent);
        let has_fishing_lines = prepared.fishing_lines != 0;
        let has_opaque_debug_lines = prepared.opaque_debug_lines != 0;
        let has_translucent_debug_lines = prepared.translucent_debug_lines != 0;
        pass.set_bind_group(0, &self.pack.terrain_bind_group, &[]);
        if block_selection_after_translucent {
            if let Some((secondary, count)) = self.block_selection.secondary_vertices() {
                pass.set_pipeline(&self.pipelines.translucent_debug_line);
                pass.set_vertex_buffer(0, secondary.slice(..));
                pass.draw(0..count, 0..1);
                stats.pipeline_switches += 1;
            }
        }
        let block_selection_vertices = block_selection_after_translucent
            .then(|| self.block_selection.vertices())
            .flatten();
        let chunk_border_vertices = self.chunk_border_outline.vertices();
        let entity_scene_vertices = self.entity_scene_outline.vertices();
        if block_selection_vertices.is_some()
            || chunk_border_vertices.is_some()
            || entity_scene_vertices.is_some()
            || has_fishing_lines
            || has_opaque_debug_lines
        {
            pass.set_pipeline(&self.pipelines.selection);
            stats.pipeline_switches += 1;
            if let Some((vertices, count)) = block_selection_vertices {
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.draw(0..count, 0..1);
                stats.selection_draw_calls += 1;
            }
            if let Some((vertices, count)) = chunk_border_vertices {
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.draw(0..count, 0..1);
                stats.chunk_border_draw_calls += 1;
            }
            if let Some((vertices, count)) = entity_scene_vertices {
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.draw(0..count, 0..1);
                stats.entity_scene_draw_calls += 1;
            }
            if has_fishing_lines {
                let buffer = self
                    .frame_entity_model_line_vertices
                    .buffer()
                    .expect("fishing-hook line vertices uploaded");
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..prepared.fishing_lines, 0..1);
                stats.entity_model_draw_calls += 1;
            }
            if has_opaque_debug_lines {
                let buffer = self
                    .frame_debug_line_opaque_vertices
                    .buffer()
                    .expect("opaque debug line vertices uploaded");
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..prepared.opaque_debug_lines, 0..1);
                stats.debug_line_draw_calls += 1;
            }
        }
        if has_translucent_debug_lines {
            let buffer = self
                .frame_debug_line_translucent_vertices
                .buffer()
                .expect("translucent debug line vertices uploaded");
            pass.set_pipeline(&self.pipelines.translucent_debug_line);
            stats.pipeline_switches += 1;
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..prepared.translucent_debug_lines, 0..1);
            stats.debug_line_draw_calls += 1;
        }
        if let Some((vertex_buffer, count)) = self.chunk_border_outline.always_on_top_vertices() {
            pass.set_pipeline(&self.pipelines.chunk_border_always_on_top);
            stats.pipeline_switches += 1;
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..count, 0..1);
            stats.chunk_border_draw_calls += 1;
        }
    }

    pub(super) fn draw_particle_target<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
        particles: &'a ParticleBatches,
        prepared: &PreparedWorldTargets,
    ) {
        if let Some(vertex_buffer) = prepared
            .translucent_particles
            .then(|| self.frame_translucent_particle_vertices.buffer())
        {
            pass.set_pipeline(&self.pipelines.translucent_particle);
            stats.pipeline_switches += 1;
            self.draw_particle_batch(
                pass,
                vertex_buffer.expect("uploaded"),
                &particles.translucent,
                stats,
            );
        }
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[crate::item_models::ItemModelDrawPhase::Solid],
            Some(&[crate::item_models::ItemModelAtlasKind::Blocks]),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[crate::item_models::ItemModelDrawPhase::Solid],
            Some(&[crate::item_models::ItemModelAtlasKind::Items]),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[
                crate::item_models::ItemModelDrawPhase::Solid,
                crate::item_models::ItemModelDrawPhase::CutoutNoCull,
                crate::item_models::ItemModelDrawPhase::CutoutZOffset,
                crate::item_models::ItemModelDrawPhase::EndPortal,
                crate::item_models::ItemModelDrawPhase::EndGateway,
                crate::item_models::ItemModelDrawPhase::Pattern,
            ],
            Some(RESIDENT_SPECIAL_ITEM_ATLASES),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[crate::item_models::ItemModelDrawPhase::Glint],
            None,
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[crate::item_models::ItemModelDrawPhase::Translucent],
            Some(&[crate::item_models::ItemModelAtlasKind::Blocks]),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[crate::item_models::ItemModelDrawPhase::Translucent],
            Some(&[crate::item_models::ItemModelAtlasKind::Items]),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[
                crate::item_models::ItemModelDrawPhase::Translucent,
                crate::item_models::ItemModelDrawPhase::TranslucentNoCull,
            ],
            Some(RESIDENT_SPECIAL_ITEM_ATLASES),
            stats,
        );
        self.draw_resident_item_models(
            pass,
            crate::item_models::ItemModelDrawTarget::Particle,
            &[crate::item_models::ItemModelDrawPhase::GlintTranslucent],
            None,
            stats,
        );
        if let (Some(index_count), Some(atlas)) = (
            prepared.experience_orbs,
            self.pack.entity_model_texture_atlas.as_ref(),
        ) {
            pass.set_pipeline(&self.pipelines.entity_model_translucent_cull);
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
            prepared.projectiles,
            self.pack.entity_model_texture_atlas.as_ref(),
        ) {
            pass.set_pipeline(&self.pipelines.entity_model_translucent_cull);
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
            prepared.elder_guardians,
            self.pack.entity_model_texture_atlas.as_ref(),
        ) {
            pass.set_pipeline(&self.pipelines.entity_model_translucent);
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

    pub(super) fn draw_weather(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut FrameDrawStats,
        prepared: &PreparedWorldTargets,
        depth_write: bool,
    ) {
        if let Some(index_count) = prepared.lightning {
            let vertex_buffer = self.frame_lightning_vertices.buffer().expect("uploaded");
            let index_buffer = self.frame_lightning_indices.buffer().expect("uploaded");
            pass.set_pipeline(&self.pipelines.lightning);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.pack.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..index_count, 0, 0..1);
            stats.weather_draw_calls += 1;
        }
        if let Some((rain, snow)) = &prepared.weather {
            let vertex_buffer = self.frame_weather_vertices.buffer().expect("uploaded");
            let index_buffer = self.frame_weather_indices.buffer().expect("uploaded");
            let pipeline = if depth_write {
                &self.pipelines.weather_depth_write
            } else {
                &self.pipelines.weather_no_depth_write
            };
            pass.set_pipeline(pipeline);
            stats.pipeline_switches += 1;
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            if !rain.is_empty() {
                if let Some(texture) = &self.pack.weather_rain_texture {
                    pass.set_bind_group(0, &texture.bind_group, &[]);
                    pass.draw_indexed(rain.clone(), 0, 0..1);
                    stats.weather_draw_calls += 1;
                }
            }
            if !snow.is_empty() {
                if let Some(texture) = &self.pack.weather_snow_texture {
                    pass.set_bind_group(0, &texture.bind_group, &[]);
                    pass.draw_indexed(snow.clone(), 0, 0..1);
                    stats.weather_draw_calls += 1;
                }
            }
        }
        // Vanilla `LevelRenderer.addWeatherPass` runs
        // `worldBorderRenderer.render(...)` after
        // `weatherEffectRenderer.render(...)`,
        // and the border draws into the weather target's color+depth when
        // that target exists. The mesh
        // indices already carry the closest-face-first sorted visible-side
        // draw list, so one indexed draw replays vanilla's
        // `drawMultipleIndexed` order.
        if let Some(index_count) = prepared.border {
            if let Some(texture) = &self.pack.world_border_texture {
                let vertex_buffer = self.frame_world_border_vertices.buffer().expect("uploaded");
                let index_buffer = self.frame_world_border_indices.buffer().expect("uploaded");
                pass.set_pipeline(&self.pipelines.world_border);
                stats.pipeline_switches += 1;
                pass.set_bind_group(0, &texture.bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..index_count, 0, 0..1);
                stats.weather_draw_calls += 1;
            }
        }
    }
}
