//! Item-model and map-surface draws shared by the world, first-person and GUI passes.

use super::*;

impl Renderer {
    pub(super) fn has_resident_map_surfaces(
        &self,
        target: crate::item_models::MapSurfaceTarget,
    ) -> bool {
        (target == crate::item_models::MapSurfaceTarget::FirstPerson
            && self.pack.map_surface_residency.background_count != 0)
            || self
                .pack
                .map_surface_residency
                .dynamic_draws
                .iter()
                .any(|draw| draw.target == target)
            || self
                .pack
                .map_surface_residency
                .decoration_draws
                .iter()
                .any(|draw| draw.target == target)
    }

    pub(super) fn draw_resident_dynamic_maps<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        target: crate::item_models::MapSurfaceTarget,
        stats: &mut FrameDrawStats,
    ) {
        let (Some(atlas), Some(instance_buffer)) = (
            &self.pack.item_frame_map_atlas,
            self.pack.map_surface_residency.dynamic_buffer(),
        ) else {
            return;
        };
        pass.set_pipeline(&self.pack.map_surface_residency.pipeline);
        pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
        pass.set_vertex_buffer(0, self.pack.map_surface_residency.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.slice(..));
        pass.set_index_buffer(
            self.pack.map_surface_residency.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        stats.pipeline_switches = stats.pipeline_switches.saturating_add(1);
        for draw in &self.pack.map_surface_residency.dynamic_draws {
            if draw.target != target {
                continue;
            }
            let Some(bind_group) = atlas.page_bind_group(draw.page, target) else {
                continue;
            };
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw_indexed(
                0..self.pack.map_surface_residency.index_count,
                0,
                draw.first_instance..draw.first_instance.saturating_add(draw.instance_count),
            );
            stats.item_model_draw_calls = stats.item_model_draw_calls.saturating_add(1);
        }
    }

    pub(super) fn draw_resident_map_backgrounds<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
    ) {
        let (Some(atlas), Some(instance_buffer)) = (
            &self.pack.first_person_map_background_atlas,
            self.pack.map_surface_residency.background_buffer(),
        ) else {
            return;
        };
        self.bind_resident_map_surface(pass, &atlas.bind_group, instance_buffer);
        pass.draw_indexed(
            0..self.pack.map_surface_residency.index_count,
            0,
            0..self.pack.map_surface_residency.background_count,
        );
        stats.pipeline_switches = stats.pipeline_switches.saturating_add(1);
        stats.item_model_draw_calls = stats.item_model_draw_calls.saturating_add(1);
    }

    pub(super) fn draw_resident_map_decorations<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        target: crate::item_models::MapSurfaceTarget,
        stats: &mut FrameDrawStats,
    ) {
        let (Some(atlas), Some(instance_buffer)) = (
            &self.pack.item_frame_map_decoration_atlas,
            self.pack.map_surface_residency.decoration_buffer(),
        ) else {
            return;
        };
        let bind_group = if target == crate::item_models::MapSurfaceTarget::FirstPerson {
            &atlas.hud3d_bind_group
        } else {
            &atlas.bind_group
        };
        self.bind_resident_map_surface(pass, bind_group, instance_buffer);
        stats.pipeline_switches = stats.pipeline_switches.saturating_add(1);
        for draw in &self.pack.map_surface_residency.decoration_draws {
            if draw.target != target {
                continue;
            }
            pass.draw_indexed(
                0..self.pack.map_surface_residency.index_count,
                0,
                draw.first_instance..draw.first_instance.saturating_add(draw.instance_count),
            );
            stats.item_model_draw_calls = stats.item_model_draw_calls.saturating_add(1);
        }
    }

    pub(super) fn has_resident_map_text(
        &self,
        target: crate::item_models::MapSurfaceTarget,
    ) -> bool {
        self.pack.item_frame_map_text_font_atlas.is_some()
            && self
                .pack
                .map_surface_residency
                .text_draws
                .iter()
                .any(|draw| draw.target == target)
    }

    pub(super) fn draw_resident_map_text<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        target: crate::item_models::MapSurfaceTarget,
        stats: &mut FrameDrawStats,
    ) {
        let (Some(atlas), Some(instance_buffer)) = (
            &self.pack.item_frame_map_text_font_atlas,
            self.pack.map_surface_residency.text_buffer(),
        ) else {
            return;
        };
        let bind_group = if target == crate::item_models::MapSurfaceTarget::FirstPerson {
            &atlas.hud3d_bind_group
        } else {
            &atlas.bind_group
        };
        self.bind_resident_map_surface(pass, bind_group, instance_buffer);
        stats.pipeline_switches = stats.pipeline_switches.saturating_add(1);
        for draw in &self.pack.map_surface_residency.text_draws {
            if draw.target != target {
                continue;
            }
            pass.draw_indexed(
                0..self.pack.map_surface_residency.index_count,
                0,
                draw.first_instance..draw.first_instance.saturating_add(draw.instance_count),
            );
            stats.item_model_draw_calls = stats.item_model_draw_calls.saturating_add(1);
        }
    }

    pub(super) fn bind_resident_map_surface<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
        instance_buffer: &'a wgpu::Buffer,
    ) {
        pass.set_pipeline(&self.pack.map_surface_residency.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
        pass.set_vertex_buffer(0, self.pack.map_surface_residency.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.slice(..));
        pass.set_index_buffer(
            self.pack.map_surface_residency.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
    }

    pub(super) fn draw_main_resident_item_models<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        stats: &mut FrameDrawStats,
    ) {
        use crate::item_models::{
            ItemModelAtlasKind as Atlas, ItemModelDrawPhase as Phase, ItemModelDrawTarget as Target,
        };

        type DrawBatch<'b> = (Target, &'b [Phase], Option<&'b [Atlas]>);
        let batches: [DrawBatch<'_>; 6] = [
            (
                Target::World,
                &[Phase::Solid, Phase::SolidNoCutout],
                Some(&[Atlas::Blocks]),
            ),
            (
                Target::World,
                &[Phase::SolidZOffsetForward],
                Some(&[Atlas::Blocks]),
            ),
            (Target::World, &[Phase::Solid], Some(&[Atlas::Items])),
            (
                Target::World,
                &[
                    Phase::Solid,
                    Phase::CutoutNoCull,
                    Phase::CutoutZOffset,
                    Phase::EndPortal,
                    Phase::EndGateway,
                    Phase::Pattern,
                ],
                Some(RESIDENT_SPECIAL_ITEM_ATLASES),
            ),
            (Target::World, &[Phase::Glint], None),
            (
                Target::BlockFeature,
                &[
                    Phase::Solid,
                    Phase::SolidNoCutout,
                    Phase::SolidZOffsetForward,
                ],
                None,
            ),
        ];
        for (target, phases, atlases) in batches {
            self.draw_resident_item_models(pass, target, phases, atlases, stats);
        }
    }

    pub(super) fn draw_resident_item_models<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        target: crate::item_models::ItemModelDrawTarget,
        phases: &[crate::item_models::ItemModelDrawPhase],
        atlases: Option<&[crate::item_models::ItemModelAtlasKind]>,
        stats: &mut FrameDrawStats,
    ) {
        if !self.pack.item_model_residency.has_draws_for_target(target) {
            return;
        }
        let Some((vertex_arena, index_arena, instance_buffer)) =
            self.pack.item_model_residency.shared_buffers()
        else {
            return;
        };
        let mut buffers_bound = false;
        let mut active_phase = None;
        let mut active_group0: Option<*const wgpu::BindGroup> = None;
        let mut active_group1: Option<*const wgpu::BindGroup> = None;
        let mut active_scissor = None;
        let full_scissor = crate::item_models::ItemModelScissorRect::new(
            0,
            0,
            self.config.width.max(1),
            self.config.height.max(1),
        )
        .expect("renderer attachment has a non-empty physical extent");
        for draw in self.pack.item_model_residency.draws_for_target(target) {
            if !phases.contains(&draw.phase)
                || atlases.is_some_and(|atlases| !atlases.contains(&draw.asset.atlas()))
            {
                continue;
            }
            let Some(owner) = self.pack.item_model_residency.draw_owner(draw) else {
                continue;
            };
            let scissor = match draw.scissor {
                Some(scissor) => {
                    // Item-model scissors are GUI-space rects; scale them to
                    // the physical attachment before clipping, mirroring the
                    // HUD sprite pass conversion.
                    let Some((x, y, width, height)) = physical_scissor_from_gui(
                        self.gui_scale,
                        self.surface_size(),
                        scissor.x(),
                        scissor.y(),
                        scissor.width(),
                        scissor.height(),
                    ) else {
                        continue;
                    };
                    let Some(scissor) =
                        crate::item_models::ItemModelScissorRect::new(x, y, width, height)
                            .and_then(|scissor| {
                                scissor
                                    .clipped_to(self.config.width.max(1), self.config.height.max(1))
                            })
                    else {
                        continue;
                    };
                    scissor
                }
                None => full_scissor,
            };
            let (group0, needs_group1) = if matches!(
                draw.phase,
                crate::item_models::ItemModelDrawPhase::Glint
                    | crate::item_models::ItemModelDrawPhase::GlintTranslucent
            ) {
                let Some(glint) = &self.pack.item_glint_texture else {
                    continue;
                };
                let bind_group = if target.is_gui() {
                    &glint.gui_bind_group
                } else if target == crate::item_models::ItemModelDrawTarget::FirstPerson {
                    &glint.hud3d_bind_group
                } else {
                    &glint.main_bind_group
                };
                (bind_group, false)
            } else {
                let bind_group = match (
                    draw.asset.atlas(),
                    target.is_gui(),
                    target == crate::item_models::ItemModelDrawTarget::FirstPerson,
                ) {
                    (crate::item_models::ItemModelAtlasKind::Blocks, false, false) => {
                        &self.pack.terrain_bind_group
                    }
                    (crate::item_models::ItemModelAtlasKind::Blocks, false, true) => {
                        &self.pack.hud3d_terrain_bind_group
                    }
                    (crate::item_models::ItemModelAtlasKind::Blocks, true, _) => {
                        &self.pack.gui_item_bind_group
                    }
                    (crate::item_models::ItemModelAtlasKind::Items, gui, first_person) => {
                        let Some(atlas) = &self.pack.item_entity_atlas else {
                            continue;
                        };
                        if gui {
                            &atlas.gui_bind_group
                        } else if first_person {
                            &atlas.hud3d_bind_group
                        } else {
                            &atlas.bind_group
                        }
                    }
                    (crate::item_models::ItemModelAtlasKind::Entity, gui, first_person) => {
                        let Some(atlas) = &self.pack.entity_model_texture_atlas else {
                            continue;
                        };
                        if gui {
                            &atlas.gui_bind_group
                        } else if first_person {
                            &atlas.hud3d_bind_group
                        } else {
                            &atlas.bind_group
                        }
                    }
                    (
                        crate::item_models::ItemModelAtlasKind::DynamicPlayerSkin,
                        gui,
                        first_person,
                    ) => {
                        let Some(atlas) = &self.entity_dynamic_player_skin_atlas else {
                            continue;
                        };
                        if gui {
                            &atlas.gui_bind_group
                        } else if first_person {
                            &atlas.hud3d_bind_group
                        } else {
                            &atlas.bind_group
                        }
                    }
                    (crate::item_models::ItemModelAtlasKind::DynamicEntity, gui, first_person) => {
                        let Some(atlas) = &self.entity_dynamic_player_texture_atlas else {
                            continue;
                        };
                        if gui {
                            &atlas.gui_bind_group
                        } else if first_person {
                            &atlas.hud3d_bind_group
                        } else {
                            &atlas.bind_group
                        }
                    }
                };
                let needs_lightmap = !matches!(
                    draw.phase,
                    crate::item_models::ItemModelDrawPhase::EndPortal
                        | crate::item_models::ItemModelDrawPhase::EndGateway
                        | crate::item_models::ItemModelDrawPhase::Outline
                );
                (bind_group, needs_lightmap)
            };
            let pipeline = match draw.phase {
                crate::item_models::ItemModelDrawPhase::Solid => {
                    &self.pipelines.resident_item_model
                }
                crate::item_models::ItemModelDrawPhase::SolidNoCutout => {
                    &self.pipelines.resident_item_model_solid_no_cutout
                }
                crate::item_models::ItemModelDrawPhase::SolidZOffsetForward => {
                    &self.pipelines.resident_item_model_z_offset_forward
                }
                crate::item_models::ItemModelDrawPhase::CutoutNoCull => {
                    &self.pipelines.resident_item_model_cutout_no_cull
                }
                crate::item_models::ItemModelDrawPhase::CutoutZOffset => {
                    &self.pipelines.resident_item_model_cutout_z_offset
                }
                crate::item_models::ItemModelDrawPhase::Pattern => {
                    &self.pipelines.resident_item_model_pattern
                }
                crate::item_models::ItemModelDrawPhase::Translucent => {
                    &self.pipelines.resident_item_model_translucent
                }
                crate::item_models::ItemModelDrawPhase::TranslucentNoCull => {
                    &self.pipelines.resident_item_model_translucent_no_cull
                }
                crate::item_models::ItemModelDrawPhase::Glint
                | crate::item_models::ItemModelDrawPhase::GlintTranslucent => {
                    &self.pipelines.resident_item_model_glint
                }
                crate::item_models::ItemModelDrawPhase::EndPortal => {
                    &self.pipelines.resident_item_model_end_portal
                }
                crate::item_models::ItemModelDrawPhase::EndGateway => {
                    &self.pipelines.resident_item_model_end_gateway
                }
                crate::item_models::ItemModelDrawPhase::Outline => {
                    &self.pipelines.resident_item_model_outline
                }
            };
            if active_phase != Some(draw.phase) {
                pass.set_pipeline(pipeline);
                active_phase = Some(draw.phase);
                if !needs_group1 {
                    active_group1 = None;
                }
                stats.pipeline_switches = stats.pipeline_switches.saturating_add(1);
            }
            let group0_ptr = std::ptr::from_ref(group0);
            if active_group0 != Some(group0_ptr) {
                pass.set_bind_group(0, group0, &[]);
                active_group0 = Some(group0_ptr);
            }
            if needs_group1 {
                let group1 = &self.lightmap.sample_bind_group;
                let group1_ptr = std::ptr::from_ref(group1);
                if active_group1 != Some(group1_ptr) {
                    pass.set_bind_group(1, group1, &[]);
                    active_group1 = Some(group1_ptr);
                }
            }
            if active_scissor != Some(scissor) {
                pass.set_scissor_rect(scissor.x(), scissor.y(), scissor.width(), scissor.height());
                active_scissor = Some(scissor);
            }
            if !buffers_bound {
                pass.set_vertex_buffer(0, vertex_arena.slice(..));
                pass.set_vertex_buffer(1, instance_buffer.slice(..));
                pass.set_index_buffer(index_arena.slice(..), wgpu::IndexFormat::Uint32);
                buffers_bound = true;
            }
            let indices = owner.first_index..owner.first_index.saturating_add(owner.index_count);
            let instances =
                draw.first_instance..draw.first_instance.saturating_add(draw.instance_count);
            pass.draw_indexed(indices, owner.base_vertex, instances);
            stats.item_model_draw_calls = stats.item_model_draw_calls.saturating_add(1);
        }
        if active_scissor.is_some_and(|scissor| scissor != full_scissor) {
            pass.set_scissor_rect(
                full_scissor.x(),
                full_scissor.y(),
                full_scissor.width(),
                full_scissor.height(),
            );
        }
    }

    pub(super) fn draw_resident_gui_map_preview<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        target: crate::item_models::MapSurfaceTarget,
    ) -> u64 {
        let mut draw_calls = 0u64;
        if let (Some(atlas), Some(instance_buffer)) = (
            &self.pack.item_frame_map_atlas,
            self.pack.map_surface_residency.dynamic_buffer(),
        ) {
            pass.set_vertex_buffer(1, instance_buffer.slice(..));
            for draw in &self.pack.map_surface_residency.dynamic_draws {
                if draw.target != target {
                    continue;
                }
                let Some(bind_group) = atlas.page_bind_group(draw.page, target) else {
                    continue;
                };
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw_indexed(
                    0..self.pack.map_surface_residency.index_count,
                    0,
                    draw.first_instance..draw.first_instance.saturating_add(draw.instance_count),
                );
                draw_calls = draw_calls.saturating_add(1);
            }
        }
        if let (Some(atlas), Some(instance_buffer)) = (
            &self.pack.item_frame_map_decoration_atlas,
            self.pack.map_surface_residency.decoration_buffer(),
        ) {
            pass.set_bind_group(0, &atlas.gui_bind_group, &[]);
            pass.set_vertex_buffer(1, instance_buffer.slice(..));
            for draw in &self.pack.map_surface_residency.decoration_draws {
                if draw.target != target {
                    continue;
                }
                pass.draw_indexed(
                    0..self.pack.map_surface_residency.index_count,
                    0,
                    draw.first_instance..draw.first_instance.saturating_add(draw.instance_count),
                );
                draw_calls = draw_calls.saturating_add(1);
            }
        }
        if let (Some(atlas), Some(instance_buffer)) = (
            &self.pack.item_frame_map_text_font_atlas,
            self.pack.map_surface_residency.text_buffer(),
        ) {
            pass.set_bind_group(0, &atlas.gui_bind_group, &[]);
            pass.set_vertex_buffer(1, instance_buffer.slice(..));
            for draw in &self.pack.map_surface_residency.text_draws {
                if draw.target != target {
                    continue;
                }
                pass.draw_indexed(
                    0..self.pack.map_surface_residency.index_count,
                    0,
                    draw.first_instance..draw.first_instance.saturating_add(draw.instance_count),
                );
                draw_calls = draw_calls.saturating_add(1);
            }
        }
        draw_calls
    }
}
