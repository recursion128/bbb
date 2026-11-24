//! Main-world geometry prepared before pass encoding, and its ordered feature draws.

use super::*;

pub(super) struct PreparedWorldFeatures {
    flame: bool,
    text: bool,
    filled_indices: u32,
    debug_text_indices: u32,
}

impl Renderer {
    pub(super) fn prepare_world_features(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> PreparedWorldFeatures {
        let filled = build_debug_filled_mesh(
            self.debug_filled_boxes
                .iter()
                .chain(self.game_test_highlight_boxes.iter()),
            &self.debug_filled_rects,
            self.camera_sort_position(),
        );
        let filled_vertices = self.frame_debug_filled_box_vertices.upload_staged(
            &self.device,
            encoder,
            self.frame_uploads.belt(),
            bytemuck::cast_slice(&filled.vertices),
        );
        let filled_indices = self.frame_debug_filled_box_indices.upload_staged(
            &self.device,
            encoder,
            self.frame_uploads.belt(),
            bytemuck::cast_slice(&filled.indices),
        );
        let filled_indices = if filled_vertices && filled_indices {
            filled.indices.len() as u32
        } else {
            0
        };
        let upload_debug_text = |renderer: &mut Self, encoder: &mut wgpu::CommandEncoder| {
            if let (Some(camera), Some(_)) = (
                renderer.camera_pose,
                renderer.pack.item_frame_map_text_font_atlas.as_ref(),
            ) {
                let mesh = build_debug_text_mesh_for_layer(
                    renderer
                        .debug_text_billboards
                        .iter()
                        .chain(renderer.game_test_highlight_text_billboards.iter()),
                    &renderer.pack.hud_font_glyphs,
                    camera,
                    false,
                );
                let vertices = renderer.frame_debug_text_vertices.upload_staged(
                    &renderer.device,
                    encoder,
                    renderer.frame_uploads.belt(),
                    bytemuck::cast_slice(&mesh.vertices),
                );
                let indices = renderer.frame_debug_text_indices.upload_staged(
                    &renderer.device,
                    encoder,
                    renderer.frame_uploads.belt(),
                    bytemuck::cast_slice(&mesh.indices),
                );
                if vertices && indices {
                    mesh.indices.len() as u32
                } else {
                    0
                }
            } else {
                0
            }
        };
        let mut debug_text_indices = upload_debug_text(self, encoder);
        let flame = self.pack.entity_flames.buffers.index_count != 0;
        let text = self.pack.item_frame_map_text_font_atlas.is_some();
        if text {
            self.prepare_sign_text();
            self.prepare_display_text();
            // Debug billboards or obfuscated sign text may lazily realize glyphs and grow the
            // dynamic font atlas; re-upload world text if the atlas texture resized.
            if self.flush_dynamic_font_atlas() {
                debug_text_indices = upload_debug_text(self, encoder);
                self.prepare_entity_name_tags(encoder);
                self.prepare_sign_text();
                self.prepare_display_text();
                self.pack
                    .map_surface_residency
                    .flush_pending_text_atlas_rescale(&self.device, &self.queue);
            }
        }
        PreparedWorldFeatures {
            flame,
            text,
            filled_indices,
            debug_text_indices,
        }
    }

    pub(super) fn has_early_world_features(&self, prepared: &PreparedWorldFeatures) -> bool {
        self.has_entity_translucent_features()
            || prepared.filled_indices != 0
            || prepared.debug_text_indices != 0
            || prepared.text
                && (self
                    .pack
                    .entity_name_tags
                    .buffers
                    .iter()
                    .any(|buffer| buffer.index_count != 0)
                    || self
                        .pack
                        .sign_text
                        .buffers
                        .iter()
                        .any(|buffer| buffer.index_count != 0)
                    || self.pack.display_text.buffers[..2]
                        .iter()
                        .any(|buffer| buffer.index_count != 0))
    }

    pub(super) fn has_late_world_features(&self, prepared: &PreparedWorldFeatures) -> bool {
        !self.block_destroy_overlays.is_empty()
            || self.block_entity_crumbling_index_count != 0
            || self.has_resident_map_surfaces(crate::item_models::MapSurfaceTarget::World)
            || self.has_resident_map_text(crate::item_models::MapSurfaceTarget::World)
            || prepared.text
                && self.pack.display_text.buffers[2..]
                    .iter()
                    .any(|buffer| buffer.index_count != 0)
    }

    fn draw_resident_world_feature<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        buffers: &'a crate::item_render::item_models::display_text::WorldTextBuffers,
        pipeline: &'a wgpu::RenderPipeline,
        bind_group: &'a wgpu::BindGroup,
        pipeline_switches: &mut u64,
        draw_calls: &mut u64,
    ) {
        if buffers.index_count == 0 {
            return;
        }
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
        pass.set_vertex_buffer(
            0,
            buffers
                .vertices
                .buffer()
                .expect("resident text vertices")
                .slice(..),
        );
        pass.set_index_buffer(
            buffers
                .indices
                .buffer()
                .expect("resident text indices")
                .slice(..),
            wgpu::IndexFormat::Uint32,
        );
        pass.draw_indexed(0..buffers.index_count, 0, 0..1);
        *pipeline_switches += 1;
        *draw_calls += 1;
    }

    pub(super) fn draw_entity_shadows<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
    ) {
        if let Some(texture) = &self.pack.entity_shadow_texture {
            self.draw_resident_world_feature(
                pass,
                &self.pack.entity_shadows.buffers,
                &self.pipelines.entity_shadow,
                &texture.bind_group,
                &mut stats.pipeline_switches,
                &mut stats.entity_model_draw_calls,
            );
        }
    }

    pub(super) fn draw_early_world_features<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
        prepared: &'a PreparedWorldFeatures,
    ) {
        self.draw_entity_translucent_features(
            pass,
            &mut stats.pipeline_switches,
            &mut stats.entity_model_draw_calls,
        );
        if prepared.text {
            let atlas = self
                .pack
                .item_frame_map_text_font_atlas
                .as_ref()
                .expect("prepared world text atlas");
            for (buffers, pipeline) in self.pack.entity_name_tags.buffers.iter().zip([
                &self.pipelines.display_text_see_through,
                &self.pipelines.display_text_background,
            ]) {
                self.draw_resident_world_feature(
                    pass,
                    buffers,
                    pipeline,
                    &atlas.bind_group,
                    &mut stats.pipeline_switches,
                    &mut stats.item_model_draw_calls,
                );
            }
            for (buffers, pipeline, group) in [
                (
                    &self.pack.sign_text.buffers[0],
                    &self.pipelines.display_text_background,
                    &atlas.bind_group,
                ),
                (
                    &self.pack.sign_text.buffers[1],
                    &self.pipelines.display_text,
                    &atlas.bind_group,
                ),
                (
                    &self.pack.sign_text.buffers[2],
                    &self.pipelines.display_text,
                    &atlas.effect_bind_group,
                ),
            ] {
                self.draw_resident_world_feature(
                    pass,
                    buffers,
                    pipeline,
                    group,
                    &mut stats.pipeline_switches,
                    &mut stats.item_model_draw_calls,
                );
            }
            for (buffers, pipeline) in self.pack.display_text.buffers[..2].iter().zip([
                &self.pipelines.display_text,
                &self.pipelines.display_text_see_through,
            ]) {
                self.draw_resident_world_feature(
                    pass,
                    buffers,
                    pipeline,
                    &atlas.bind_group,
                    &mut stats.pipeline_switches,
                    &mut stats.item_model_draw_calls,
                );
            }
        }
        if prepared.filled_indices != 0 {
            pass.set_pipeline(&self.pipelines.debug_filled_box);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.pack.terrain_bind_group, &[]);
            pass.set_vertex_buffer(
                0,
                self.frame_debug_filled_box_vertices
                    .buffer()
                    .expect("prepared filled vertices")
                    .slice(..),
            );
            pass.set_index_buffer(
                self.frame_debug_filled_box_indices
                    .buffer()
                    .expect("prepared filled indices")
                    .slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..prepared.filled_indices, 0, 0..1);
            stats.debug_filled_box_draw_calls += 1;
        }
        if prepared.debug_text_indices != 0 {
            let atlas = self
                .pack
                .item_frame_map_text_font_atlas
                .as_ref()
                .expect("prepared debug text atlas");
            pass.set_pipeline(&self.pipelines.debug_text);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(
                0,
                self.frame_debug_text_vertices
                    .buffer()
                    .expect("prepared debug text vertices")
                    .slice(..),
            );
            pass.set_index_buffer(
                self.frame_debug_text_indices
                    .buffer()
                    .expect("prepared debug text indices")
                    .slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..prepared.debug_text_indices, 0, 0..1);
            stats.debug_text_draw_calls += 1;
        }
    }

    pub(super) fn draw_ordered_world_text<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
        prepared: &'a PreparedWorldFeatures,
    ) {
        let target = crate::item_models::MapSurfaceTarget::World;
        if self
            .pack
            .map_surface_residency
            .dynamic_draws
            .iter()
            .any(|draw| draw.target == target)
        {
            self.draw_resident_dynamic_maps(pass, target, stats);
        }
        if self
            .pack
            .map_surface_residency
            .decoration_draws
            .iter()
            .any(|draw| draw.target == target)
        {
            self.draw_resident_map_decorations(pass, target, stats);
        }
        if prepared.text {
            let atlas = self
                .pack
                .item_frame_map_text_font_atlas
                .as_ref()
                .expect("prepared world text atlas");
            for (buffers, pipeline) in self.pack.display_text.buffers[2..4].iter().zip([
                &self.pipelines.display_text_background,
                &self.pipelines.display_text_see_through,
            ]) {
                self.draw_resident_world_feature(
                    pass,
                    buffers,
                    pipeline,
                    &atlas.bind_group,
                    &mut stats.pipeline_switches,
                    &mut stats.item_model_draw_calls,
                );
            }
        }
        if self.has_resident_map_text(target) {
            self.draw_resident_map_text(pass, target, stats);
        }
        if prepared.text {
            let atlas = self
                .pack
                .item_frame_map_text_font_atlas
                .as_ref()
                .expect("prepared world text atlas");
            for (buffers, pipeline) in self.pack.display_text.buffers[4..].iter().zip([
                &self.pipelines.display_text,
                &self.pipelines.display_text_see_through,
            ]) {
                self.draw_resident_world_feature(
                    pass,
                    buffers,
                    pipeline,
                    &atlas.bind_group,
                    &mut stats.pipeline_switches,
                    &mut stats.item_model_draw_calls,
                );
            }
        }
    }
    pub(super) fn draw_main_world<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
        prepared: &'a PreparedWorldFeatures,
    ) {
        let (switches, draws) = self.sky.draw(
            pass,
            &self.pack.terrain_bind_group,
            &self.pipelines.sky,
            self.pack.end_sky_texture.as_ref(),
            self.pack.celestial_atlas.as_ref(),
        );
        stats.pipeline_switches += switches;
        stats.sky_draw_calls += draws;

        // Vanilla 26.1 renders ChunkSectionLayerGroup.OPAQUE as SOLID then CUTOUT
        // before feature submissions; keep both terrain layers ahead of entity draws.
        self.draw_terrain_layer(pass, TerrainLayer::Solid, stats);
        self.draw_terrain_layer(pass, TerrainLayer::Cutout, stats);
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
        let (switches, draws) = (
            &mut stats.pipeline_switches,
            &mut stats.entity_model_draw_calls,
        );
        let meshes = &self.pack.entity_meshes;
        draw_mesh_lanes(
            pass,
            &[
                super::mesh_lanes::entity_surface_lane(
                    &meshes[EntityMeshLane::Solid],
                    &self.pipelines.entity_model_solid,
                    self.entity_pose_pipelines.as_ref().map(|p| &p.solid),
                    atlas,
                    lightmap,
                ),
                super::mesh_lanes::entity_surface_lane(
                    &meshes[EntityMeshLane::TexturedCull],
                    &self.pipelines.entity_model_textured_cull,
                    self.entity_pose_pipelines.as_ref().map(|p| &p.cutout_cull),
                    atlas,
                    lightmap,
                ),
                super::mesh_lanes::entity_surface_lane(
                    &meshes[EntityMeshLane::Textured],
                    &self.pipelines.entity_model_textured,
                    self.entity_pose_pipelines.as_ref().map(|p| &p.cutout),
                    atlas,
                    lightmap,
                ),
            ],
            switches,
            draws,
        );
        let mut draw_state = super::entity_features::EntityDrawStateCache::default();
        for draw in &meshes.opaque_beam_draws {
            self.draw_entity_geometry_range(pass, *draw, &mut draw_state, switches, draws);
        }
        let solid = &self.pipelines.entity_model_solid;
        let cutout_cull = &self.pipelines.entity_model_textured_cull;
        let cutout = &self.pipelines.entity_model_textured;
        let solid_z_offset = &self.pipelines.entity_model_solid_z_offset_forward;
        let cutout_z_offset = &self.pipelines.entity_model_cutout_z_offset;
        let armor_cutout = &self.pipelines.entity_model_armor_cutout;
        let armor_decal = &self.pipelines.entity_model_armor_decal;
        draw_mesh_lanes(
            pass,
            &[
                lane(
                    &meshes[EntityMeshLane::SolidZOffsetForward],
                    solid_z_offset,
                    atlas,
                    lightmap,
                ),
                // Vanilla `RenderTypes.entityCutoutDissolve` (the dying ender dragon body):
                // opaque cutout family, drawn through the dedicated DISSOLVE-mask pipeline.
                lane(
                    &meshes[EntityMeshLane::Dissolve],
                    &self.pipelines.entity_model_dissolve,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::CutoutZOffset],
                    cutout_z_offset,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::ArmorCutout],
                    armor_cutout,
                    atlas,
                    lightmap,
                ),
                lane(&meshes[EntityMeshLane::SkinSolid], solid, skin, lightmap),
                lane(
                    &meshes[EntityMeshLane::SkinCutoutCull],
                    cutout_cull,
                    skin,
                    lightmap,
                ),
                lane(&meshes[EntityMeshLane::SkinCutout], cutout, skin, lightmap),
                lane(
                    &meshes[EntityMeshLane::SkinSolidZOffsetForward],
                    solid_z_offset,
                    skin,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::SkinCutoutZOffset],
                    cutout_z_offset,
                    skin,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureSolid],
                    solid,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureCutoutCull],
                    cutout_cull,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureCutout],
                    cutout,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureSolidZOffsetForward],
                    solid_z_offset,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureCutoutZOffset],
                    cutout_z_offset,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureArmorCutout],
                    armor_cutout,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::WaterMask],
                    &self.pipelines.entity_model_water_mask,
                    Some(&self.pack.terrain_bind_group),
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::ArmorEntityGlint],
                    &self.pipelines.entity_model_armor_entity_glint,
                    self.pack
                        .armor_glint_texture
                        .as_ref()
                        .map(|glint| &glint.main_bind_group),
                    None,
                ),
                lane(
                    &meshes[EntityMeshLane::ArmorTrim],
                    armor_cutout,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureArmorTrim],
                    armor_cutout,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::ArmorTrimDecal],
                    armor_decal,
                    atlas,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::PlayerTextureArmorTrimDecal],
                    armor_decal,
                    texture,
                    lightmap,
                ),
                lane(
                    &meshes[EntityMeshLane::EntityGlint],
                    &self.pipelines.entity_model_entity_glint,
                    self.pack
                        .item_glint_texture
                        .as_ref()
                        .map(|glint| &glint.main_bind_group),
                    None,
                ),
            ],
            switches,
            draws,
        );

        // Vanilla `FeatureRenderDispatcher.renderSolidFeatures` renders
        // `FlameFeatureRenderer` immediately after model/model-part features
        // and before leash/item/block/custom features. These quads sample the
        // blocks atlas through `Sheets.cutoutBlockSheet()`.
        if prepared.flame {
            self.draw_resident_world_feature(
                pass,
                &self.pack.entity_flames.buffers,
                &self.pipelines.item_model,
                &self.pack.terrain_bind_group,
                &mut stats.pipeline_switches,
                &mut stats.entity_model_draw_calls,
            );
        }

        // Vanilla solid item features are part of FeatureRenderDispatcher.renderSolidFeatures()
        // inside the main pass: after model/model-part features and before target depth copies,
        // translucent terrain, entity-outline post-chain work, and clouds. GUI item icons use
        // the later HUD item pass instead.
        self.draw_main_resident_item_models(pass, stats);
    }

    pub(super) fn draw_block_destroy_overlays(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut FrameDrawStats,
    ) {
        let block_overlays = self.block_destroy_overlays.buffers();
        if block_overlays.is_none() && self.block_entity_crumbling_index_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipelines.block_destroy);
        stats.pipeline_switches += 1;
        pass.set_bind_group(0, &self.pack.terrain_bind_group, &[]);
        if let Some((vertex_buffer, index_buffer, index_count)) = block_overlays {
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..index_count, 0, 0..1);
            stats.block_destroy_overlay_draw_calls += 1;
        }
        if self.block_entity_crumbling_index_count > 0 {
            let vertices = self
                .frame_block_entity_crumbling_vertices
                .buffer()
                .expect("positive block-entity crumbling count requires uploaded vertices");
            let indices = self
                .frame_block_entity_crumbling_indices
                .buffer()
                .expect("positive block-entity crumbling count requires uploaded indices");
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.block_entity_crumbling_index_count, 0, 0..1);
            stats.block_destroy_overlay_draw_calls += 1;
        }
    }

    pub(super) fn draw_terrain_layer(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        layer: TerrainLayer,
        stats: &mut FrameDrawStats,
    ) {
        let draws = self.terrain.draw(
            &self.device,
            pass,
            layer,
            self.pipelines.terrain.get(layer),
            [
                &self.pack.terrain_bind_group,
                &self.lightmap.sample_bind_group,
            ],
            self.config.format,
            1,
        ) as u64;
        stats.pipeline_switches += u64::from(draws > 0);
        *match layer {
            TerrainLayer::Solid => &mut stats.opaque_draw_calls,
            TerrainLayer::Cutout => &mut stats.cutout_draw_calls,
            TerrainLayer::Translucent => &mut stats.translucent_draw_calls,
        } += draws;
        if matches!(layer, TerrainLayer::Solid)
            && self.lod_terrain.draw(
                pass,
                &self.pipelines.lod_terrain,
                &self.pack.terrain_bind_group,
                &self.lightmap.sample_bind_group,
            )
        {
            stats.pipeline_switches += 1;
            stats.opaque_draw_calls += 1;
        }
    }

    pub(super) fn entity_outline_target_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) {
        if self.has_entity_outline_content() {
            let outline = self.targets.entity_outline();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(ENTITY_OUTLINE_TARGET_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &outline.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &outline.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
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
            draw_mesh_lanes(
                &mut pass,
                &[
                    lane(
                        &self.pack.entity_meshes[EntityMeshLane::Outline],
                        &self.pipelines.entity_model_outline,
                        atlas,
                        None,
                    ),
                    lane(
                        &self.pack.entity_meshes[EntityMeshLane::OutlineCull],
                        &self.pipelines.entity_model_outline_cull,
                        atlas,
                        None,
                    ),
                    lane(
                        &self.pack.entity_meshes[EntityMeshLane::SkinOutline],
                        &self.pipelines.entity_model_outline,
                        skin,
                        None,
                    ),
                    lane(
                        &self.pack.entity_meshes[EntityMeshLane::SkinOutlineCull],
                        &self.pipelines.entity_model_outline_cull,
                        skin,
                        None,
                    ),
                    lane(
                        &self.pack.entity_meshes[EntityMeshLane::PlayerTextureOutline],
                        &self.pipelines.entity_model_outline,
                        texture,
                        None,
                    ),
                    lane(
                        &self.pack.entity_meshes[EntityMeshLane::PlayerTextureOutlineCull],
                        &self.pipelines.entity_model_outline_cull,
                        texture,
                        None,
                    ),
                ],
                &mut stats.pipeline_switches,
                &mut stats.entity_model_draw_calls,
            );
            self.draw_resident_item_models(
                &mut pass,
                crate::item_models::ItemModelDrawTarget::EntityOutline,
                &[crate::item_models::ItemModelDrawPhase::Outline],
                None,
                stats,
            );
        }
    }

    pub(super) fn has_entity_outline_content(&self) -> bool {
        let has_textured_meshes = (self.pack.entity_model_texture_atlas.is_some()
            && self
                .pack
                .entity_meshes
                .has_any(&[EntityMeshLane::Outline, EntityMeshLane::OutlineCull]))
            || (self.entity_dynamic_player_skin_atlas.is_some()
                && self
                    .pack
                    .entity_meshes
                    .has_any(&[EntityMeshLane::SkinOutline, EntityMeshLane::SkinOutlineCull]))
            || (self.entity_dynamic_player_texture_atlas.is_some()
                && self.pack.entity_meshes.has_any(&[
                    EntityMeshLane::PlayerTextureOutline,
                    EntityMeshLane::PlayerTextureOutlineCull,
                ]));
        has_textured_meshes
            || self
                .pack
                .item_model_residency
                .draws_for_target(crate::item_models::ItemModelDrawTarget::EntityOutline)
                .any(|draw| draw.phase == crate::item_models::ItemModelDrawPhase::Outline)
    }
}
