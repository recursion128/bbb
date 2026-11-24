//! Entity feature draws: sorted textured/translucent ranges, portals, beams and the translucent lanes.

use super::*;

#[derive(Default)]
pub(super) struct EntityDrawStateCache {
    pipeline: Option<*const wgpu::RenderPipeline>,
    group0: Option<*const wgpu::BindGroup>,
    group1: Option<*const wgpu::BindGroup>,
    vertex_buffer: Option<*const wgpu::Buffer>,
    index_buffer: Option<*const wgpu::Buffer>,
}

struct EntityMeshDraw<'a> {
    mesh: &'a EntityModelMeshGpu,
    index_start: u32,
    index_end: u32,
    pipeline: &'a wgpu::RenderPipeline,
    group0: &'a wgpu::BindGroup,
    group1: Option<&'a wgpu::BindGroup>,
}

impl EntityDrawStateCache {
    fn emit<'a>(
        &mut self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityMeshDraw<'a>,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        let pipeline_ptr = std::ptr::from_ref(draw.pipeline);
        if self.pipeline != Some(pipeline_ptr) {
            pass.set_pipeline(draw.pipeline);
            *pipeline_switches += 1;
            self.pipeline = Some(pipeline_ptr);
            self.group0 = None;
            self.group1 = None;
        }
        let group0_ptr = std::ptr::from_ref(draw.group0);
        if self.group0 != Some(group0_ptr) {
            pass.set_bind_group(0, draw.group0, &[]);
            self.group0 = Some(group0_ptr);
            self.group1 = None;
        }
        if let Some(group1) = draw.group1 {
            let group1_ptr = std::ptr::from_ref(group1);
            if self.group1 != Some(group1_ptr) {
                pass.set_bind_group(1, group1, &[]);
                self.group1 = Some(group1_ptr);
            }
        }
        let vb_ptr = std::ptr::from_ref(&draw.mesh.vertex_buffer);
        if self.vertex_buffer != Some(vb_ptr) {
            pass.set_vertex_buffer(0, draw.mesh.vertex_buffer.slice(..));
            self.vertex_buffer = Some(vb_ptr);
        }
        let ib_ptr = std::ptr::from_ref(&draw.mesh.index_buffer);
        if self.index_buffer != Some(ib_ptr) {
            pass.set_index_buffer(draw.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            self.index_buffer = Some(ib_ptr);
        }
        pass.draw_indexed(draw.index_start..draw.index_end, 0, 0..1);
        *entity_model_draw_calls += 1;
    }
}

impl Renderer {
    pub(super) fn has_entity_translucent_features(&self) -> bool {
        (self.pack.entity_model_texture_atlas.is_some()
            && self.pack.entity_meshes.has_any(&[
                EntityMeshLane::Translucent,
                EntityMeshLane::ArmorTranslucent,
                EntityMeshLane::TranslucentEmissive,
                EntityMeshLane::Eyes,
                EntityMeshLane::DragonRays,
                EntityMeshLane::DragonRaysDepth,
                EntityMeshLane::Leash,
                EntityMeshLane::EndPortal,
                EntityMeshLane::EndGateway,
                EntityMeshLane::Scroll,
                EntityMeshLane::ScrollAdditive,
                EntityMeshLane::BeaconBeamTranslucent,
            ]))
            || (self.entity_dynamic_player_skin_atlas.is_some()
                && self.pack.entity_meshes[EntityMeshLane::SkinTranslucent].is_some())
            || (self.entity_dynamic_player_texture_atlas.is_some()
                && self.pack.entity_meshes[EntityMeshLane::PlayerTextureTranslucent].is_some())
    }

    pub(super) fn draw_entity_item_entity_target_features<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        if !self.pack.entity_meshes.sorted_item_entity_draws.is_empty() {
            let mut draw_state = EntityDrawStateCache::default();
            for draw in &self.pack.entity_meshes.sorted_item_entity_draws {
                self.draw_entity_textured_range(
                    pass,
                    *draw,
                    true,
                    &mut draw_state,
                    pipeline_switches,
                    entity_model_draw_calls,
                );
            }
            return;
        }

        let lightmap = Some(&self.lightmap.sample_bind_group);
        let atlas = self
            .pack
            .entity_model_texture_atlas
            .as_ref()
            .map(|atlas| &atlas.bind_group);
        let skin = self
            .entity_dynamic_player_skin_atlas
            .as_ref()
            .map(|atlas| &atlas.bind_group);
        let texture = self
            .entity_dynamic_player_texture_atlas
            .as_ref()
            .map(|atlas| &atlas.bind_group);
        let translucent = &self.pipelines.entity_model_translucent;
        let translucent_cull = &self.pipelines.entity_model_translucent_cull;
        let meshes = &self.pack.entity_meshes;
        draw_mesh_lanes(
            pass,
            &[
                lane(
                    &meshes[EntityMeshLane::ItemEntityTranslucent],
                    translucent,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::ItemEntityTranslucentCull],
                    translucent_cull,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::SkinItemEntityTranslucent],
                    translucent,
                    skin,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::SkinItemEntityTranslucentCull],
                    translucent_cull,
                    skin,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureItemEntityTranslucent],
                    translucent,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureItemEntityTranslucentCull],
                    translucent_cull,
                    texture,
                    lightmap,
                ),
            ],
            pipeline_switches,
            entity_model_draw_calls,
        );
    }

    pub(super) fn draw_entity_textured_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityModelTexturedDrawRange,
        item_entity_target: bool,
        draw_state: &mut EntityDrawStateCache,
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
        let uses_banner_pattern_pipeline =
            draw.render_type == EntityModelLayerRenderType::BannerPattern;
        let uses_eyes_pipeline = draw.render_type == EntityModelLayerRenderType::Eyes;
        let pipeline = if uses_translucent_emissive_pipeline {
            &self.pipelines.entity_model_translucent_emissive
        } else if uses_armor_translucent_pipeline {
            &self.pipelines.entity_model_armor_translucent
        } else if uses_banner_pattern_pipeline {
            &self.pipelines.entity_model_banner_pattern
        } else if uses_eyes_pipeline {
            &self.pipelines.entity_model_eyes
        } else if draw.surface_cull {
            &self.pipelines.entity_model_translucent_cull
        } else {
            &self.pipelines.entity_model_translucent
        };
        let group1 = (!uses_translucent_emissive_pipeline && !uses_eyes_pipeline)
            .then_some(&self.lightmap.sample_bind_group);
        draw_state.emit(
            pass,
            EntityMeshDraw {
                mesh,
                index_start: draw.index_start,
                index_end,
                pipeline,
                group0: bind_group,
                group1,
            },
            pipeline_switches,
            entity_model_draw_calls,
        );
    }

    pub(super) fn entity_textured_range_resources(
        &self,
        draw: EntityModelTexturedDrawRange,
        item_entity_target: bool,
    ) -> Option<(&EntityModelMeshGpu, &wgpu::BindGroup)> {
        let meshes = &self.pack.entity_meshes;
        if draw.render_type == EntityModelLayerRenderType::BannerPattern {
            if item_entity_target || !draw.surface_cull {
                return None;
            }
            return match draw.atlas {
                EntityModelTexturedDrawAtlas::Static => Some((
                    meshes[EntityMeshLane::Translucent].as_ref()?,
                    &self.pack.entity_model_texture_atlas.as_ref()?.bind_group,
                )),
                EntityModelTexturedDrawAtlas::DynamicPlayerTexture => Some((
                    meshes[EntityMeshLane::PlayerTextureTranslucent].as_ref()?,
                    &self
                        .entity_dynamic_player_texture_atlas
                        .as_ref()?
                        .bind_group,
                )),
                EntityModelTexturedDrawAtlas::DynamicPlayerSkin => None,
            };
        }
        if draw.render_type == EntityModelLayerRenderType::EntityTranslucentEmissive {
            if item_entity_target
                || draw.atlas != EntityModelTexturedDrawAtlas::Static
                || draw.surface_cull
            {
                return None;
            }
            return Some((
                meshes[EntityMeshLane::TranslucentEmissive].as_ref()?,
                &self.pack.entity_model_texture_atlas.as_ref()?.bind_group,
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
                meshes[EntityMeshLane::ArmorTranslucent].as_ref()?,
                &self.pack.entity_model_texture_atlas.as_ref()?.bind_group,
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
                meshes[EntityMeshLane::Eyes].as_ref()?,
                &self.pack.entity_model_texture_atlas.as_ref()?.bind_group,
            ));
        }
        match (draw.atlas, item_entity_target, draw.surface_cull) {
            (EntityModelTexturedDrawAtlas::Static, false, false) => Some((
                meshes[EntityMeshLane::Translucent].as_ref()?,
                &self.pack.entity_model_texture_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerSkin, false, false) => Some((
                meshes[EntityMeshLane::SkinTranslucent].as_ref()?,
                &self.entity_dynamic_player_skin_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerTexture, false, false) => Some((
                meshes[EntityMeshLane::PlayerTextureTranslucent].as_ref()?,
                &self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()?
                    .bind_group,
            )),
            (EntityModelTexturedDrawAtlas::Static, true, false) => Some((
                meshes[EntityMeshLane::ItemEntityTranslucent].as_ref()?,
                &self.pack.entity_model_texture_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::Static, true, true) => Some((
                meshes[EntityMeshLane::ItemEntityTranslucentCull].as_ref()?,
                &self.pack.entity_model_texture_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerSkin, true, false) => Some((
                meshes[EntityMeshLane::SkinItemEntityTranslucent].as_ref()?,
                &self.entity_dynamic_player_skin_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerSkin, true, true) => Some((
                meshes[EntityMeshLane::SkinItemEntityTranslucentCull].as_ref()?,
                &self.entity_dynamic_player_skin_atlas.as_ref()?.bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerTexture, true, false) => Some((
                meshes[EntityMeshLane::PlayerTextureItemEntityTranslucent].as_ref()?,
                &self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()?
                    .bind_group,
            )),
            (EntityModelTexturedDrawAtlas::DynamicPlayerTexture, true, true) => Some((
                meshes[EntityMeshLane::PlayerTextureItemEntityTranslucentCull].as_ref()?,
                &self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()?
                    .bind_group,
            )),
            _ => None,
        }
    }

    pub(super) fn draw_entity_main_translucent_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityModelTranslucentDrawRange,
        draw_state: &mut EntityDrawStateCache,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        match draw {
            EntityModelTranslucentDrawRange::Textured(draw) => self.draw_entity_textured_range(
                pass,
                draw,
                false,
                draw_state,
                pipeline_switches,
                entity_model_draw_calls,
            ),
            EntityModelTranslucentDrawRange::Geometry(draw) => self.draw_entity_geometry_range(
                pass,
                draw,
                draw_state,
                pipeline_switches,
                entity_model_draw_calls,
            ),
        }
    }

    pub(super) fn draw_entity_geometry_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: EntityModelDrawRange,
        draw_state: &mut EntityDrawStateCache,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        let atlas = self
            .pack
            .entity_model_texture_atlas
            .as_ref()
            .map(|atlas| &atlas.bind_group);
        let terrain = Some(&self.pack.terrain_bind_group);
        let lightmap = Some(&self.lightmap.sample_bind_group);
        let meshes = &self.pack.entity_meshes;
        let (mesh, pipeline, group0, group1) = match draw.render_type {
            EntityModelLayerRenderType::EndPortal => (
                meshes[EntityMeshLane::EndPortal].as_ref(),
                &self.pipelines.entity_model_end_portal,
                atlas,
                None,
            ),
            EntityModelLayerRenderType::EndGateway => (
                meshes[EntityMeshLane::EndGateway].as_ref(),
                &self.pipelines.entity_model_end_gateway,
                atlas,
                None,
            ),
            EntityModelLayerRenderType::DragonRays => (
                meshes[EntityMeshLane::DragonRays].as_ref(),
                &self.pipelines.entity_model_dragon_rays,
                terrain,
                None,
            ),
            EntityModelLayerRenderType::DragonRaysDepth => (
                meshes[EntityMeshLane::DragonRaysDepth].as_ref(),
                &self.pipelines.entity_model_dragon_rays_depth,
                terrain,
                None,
            ),
            EntityModelLayerRenderType::Leash => (
                meshes[EntityMeshLane::Leash].as_ref(),
                &self.pipelines.entity_model_leash,
                terrain,
                lightmap,
            ),
            EntityModelLayerRenderType::BreezeWind => (
                meshes[EntityMeshLane::Scroll].as_ref(),
                &self.pipelines.entity_model_scroll,
                atlas,
                lightmap,
            ),
            EntityModelLayerRenderType::EnergySwirl => (
                meshes[EntityMeshLane::ScrollAdditive].as_ref(),
                &self.pipelines.entity_model_scroll_additive,
                atlas,
                None,
            ),
            EntityModelLayerRenderType::BeaconBeamTranslucent => (
                meshes[EntityMeshLane::BeaconBeamTranslucent].as_ref(),
                &self.pipelines.entity_model_beacon_beam_translucent,
                atlas,
                None,
            ),
            EntityModelLayerRenderType::EntityCutout => (
                meshes[EntityMeshLane::GuardianBeam].as_ref(),
                &self.pipelines.entity_model_guardian_beam,
                atlas,
                lightmap,
            ),
            EntityModelLayerRenderType::EndCrystalBeam => (
                meshes[EntityMeshLane::EndCrystalBeam].as_ref(),
                &self.pipelines.entity_model_end_crystal_beam,
                atlas,
                lightmap,
            ),
            EntityModelLayerRenderType::BeaconBeamOpaque => (
                meshes[EntityMeshLane::BeaconBeamOpaque].as_ref(),
                &self.pipelines.entity_model_beacon_beam_opaque,
                atlas,
                None,
            ),
            _ => return,
        };
        let (Some(mesh), Some(group0)) = (mesh, group0) else {
            return;
        };
        let index_end = draw.index_start.saturating_add(draw.index_count);
        if draw.index_count == 0 || index_end > mesh.index_count {
            return;
        }
        draw_state.emit(
            pass,
            EntityMeshDraw {
                mesh,
                index_start: draw.index_start,
                index_end,
                pipeline,
                group0,
                group1,
            },
            pipeline_switches,
            entity_model_draw_calls,
        );
    }

    pub(super) fn draw_entity_translucent_features<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline_switches: &mut u64,
        entity_model_draw_calls: &mut u64,
    ) {
        if !self
            .pack
            .entity_meshes
            .sorted_main_translucent_draws
            .is_empty()
        {
            let mut draw_state = EntityDrawStateCache::default();
            for draw in &self.pack.entity_meshes.sorted_main_translucent_draws {
                self.draw_entity_main_translucent_range(
                    pass,
                    *draw,
                    &mut draw_state,
                    pipeline_switches,
                    entity_model_draw_calls,
                );
            }
            return;
        }
        let lightmap = Some(&self.lightmap.sample_bind_group);
        let atlas = self
            .pack
            .entity_model_texture_atlas
            .as_ref()
            .map(|atlas| &atlas.bind_group);
        let translucent = &self.pipelines.entity_model_translucent;
        let meshes = &self.pack.entity_meshes;
        draw_mesh_lanes(
            pass,
            &[
                lane(
                    &meshes[EntityMeshLane::Translucent],
                    translucent,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::ArmorTranslucent],
                    &self.pipelines.entity_model_armor_translucent,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::TranslucentEmissive],
                    &self.pipelines.entity_model_translucent_emissive,
                    atlas,
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::SkinTranslucent],
                    translucent,
                    self.entity_dynamic_player_skin_atlas
                        .as_ref()
                        .map(|atlas| &atlas.bind_group),
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureTranslucent],
                    translucent,
                    self.entity_dynamic_player_texture_atlas
                        .as_ref()
                        .map(|atlas| &atlas.bind_group),
                    lightmap,
                ),
            ],
            pipeline_switches,
            entity_model_draw_calls,
        );
        let terrain = Some(&self.pack.terrain_bind_group);
        draw_mesh_lanes(
            pass,
            &[
                lane(
                    &meshes[EntityMeshLane::Eyes],
                    &self.pipelines.entity_model_eyes,
                    atlas,
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::DragonRays],
                    &self.pipelines.entity_model_dragon_rays,
                    terrain,
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::DragonRaysDepth],
                    &self.pipelines.entity_model_dragon_rays_depth,
                    terrain,
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::Leash],
                    &self.pipelines.entity_model_leash,
                    terrain,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::EndPortal],
                    &self.pipelines.entity_model_end_portal,
                    atlas,
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::EndGateway],
                    &self.pipelines.entity_model_end_gateway,
                    atlas,
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::Scroll],
                    &self.pipelines.entity_model_scroll,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::ScrollAdditive],
                    &self.pipelines.entity_model_scroll_additive,
                    atlas,
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::BeaconBeamTranslucent],
                    &self.pipelines.entity_model_beacon_beam_translucent,
                    atlas,
                    None,
                ),
            ],
            pipeline_switches,
            entity_model_draw_calls,
        );
    }
}
