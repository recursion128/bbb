use std::path::Path;

use anyhow::Result;
use winit::dpi::PhysicalSize;

use crate::frame_flight::FrameTarget;
use crate::{
    clouds::CloudShape,
    debug_filled_boxes::build_debug_filled_mesh,
    debug_text_billboards::build_debug_text_mesh_for_layer,
    entity_models::{
        fishing_hook_line_segments, upload_elder_guardian_particle_textured_mesh,
        upload_experience_orb_pickup_particle_textured_mesh,
        upload_projectile_pickup_particle_textured_mesh, EntityModelDrawRange,
        EntityModelLayerRenderType, EntityModelMeshGpu, EntityModelTexturedDrawAtlas,
        EntityModelTexturedDrawRange, EntityModelTranslucentDrawRange,
    },
    gpu_timing::GpuFrameStage,
    particles::{ParticleBatch, ParticleBatches, ParticleTextureAtlasKind},
    post_effect_graph::{PostEffectGlobals, PostEffectTransparencyTargets, VanillaPostEffect},
    renderer::{EntityMeshLane, TerrainLayer},
    selection::{debug_line_vertex_batches, selection_lines_vertices, BlockSelectionPhase},
    weather::{build_lightning_mesh, build_weather_mesh},
    world_border::build_world_border_mesh,
    Renderer,
};

// GuiRenderer multiplies clips by the integer GUI scale.
fn physical_scissor_from_gui(
    gui_scale: u32,
    physical: PhysicalSize<u32>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let scale = gui_scale.max(1);
    let left = x.saturating_mul(scale).min(physical.width);
    let top = y.saturating_mul(scale).min(physical.height);
    let right = x
        .saturating_add(width)
        .saturating_mul(scale)
        .min(physical.width);
    let bottom = y
        .saturating_add(height)
        .saturating_mul(scale)
        .min(physical.height);
    (left < right && top < bottom).then_some((left, top, right - left, bottom - top))
}

mod entity_features;
mod hud_commands;
mod hud_passes;
mod item_model_passes;
mod mesh_lanes;
mod post_passes;
mod translucent_targets;
mod world_features;
mod world_passes;

use mesh_lanes::{draw_mesh_lanes, lane};
use post_passes::PresentationPrefix;

const RESIDENT_SPECIAL_ITEM_ATLASES: &[crate::item_models::ItemModelAtlasKind] = &[
    crate::item_models::ItemModelAtlasKind::Entity,
    crate::item_models::ItemModelAtlasKind::DynamicPlayerSkin,
    crate::item_models::ItemModelAtlasKind::DynamicEntity,
];
const ENTITY_OUTLINE_TARGET_PASS_LABEL: &str = "bbb-client-entity-outline-target-pass";
const ENTITY_OUTLINE_COMPOSITE_PASS_LABEL: &str = "bbb-client-entity-outline-composite-pass";
const CLOUDS_PASS_LABEL: &str = "bbb-client-clouds-pass";
const ENTITY_TRANSLUCENT_FEATURE_PASS_LABEL: &str = "bbb-client-entity-translucent-feature-pass";
const DEBUG_ALWAYS_ON_TOP_TEXT_PASS_LABEL: &str = "bbb-client-debug-always-on-top-text-pass";
const TRANSLUCENT_TARGET_PASS_LABEL: &str = "bbb-client-translucent-target-pass";
const ITEM_ENTITY_TARGET_PASS_LABEL: &str = "bbb-client-item-entity-target-pass";
const ITEM_ENTITY_LINE_TARGET_PASS_LABEL: &str = "bbb-client-item-entity-line-target-pass";
const BLOCK_SELECTION_BEFORE_TRANSLUCENT_PASS_LABEL: &str =
    "bbb-client-block-selection-before-translucent-pass";
const PARTICLE_TARGET_PASS_LABEL: &str = "bbb-client-particle-target-pass";
const WEATHER_TARGET_PASS_LABEL: &str = "bbb-client-weather-target-pass";
const FIRST_PERSON_ITEM_PASS_LABEL: &str = "bbb-client-first-person-item-pass";
const WIN_SCREEN_PORTAL_PASS_LABEL: &str = "bbb-client-win-screen-portal-pass";
const WIN_SCREEN_VIGNETTE_PASS_LABEL: &str = "bbb-client-win-screen-vignette-pass";
const LIGHTMAP_PASS_LABEL: &str = "bbb-client-lightmap-pass";
const TRANSPARENCY_COMBINE_PASS_LABEL: &str = "bbb-client-transparency-combine-pass";
const TRANSPARENCY_GRAPH_BLIT_PASS_LABEL: &str = "bbb-client-transparency-graph-blit-pass";
const TRANSPARENCY_BLIT_PASS_LABEL: &str = "bbb-client-transparency-blit-pass";

struct FrameColor<'a> {
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
}

impl FrameColor<'_> {
    fn begin_pass(
        &mut self,
        label: &str,
        load: wgpu::LoadOp<wgpu::Color>,
        depth: Option<(&wgpu::TextureView, wgpu::LoadOp<f32>)>,
    ) -> wgpu::RenderPass<'_> {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth.map(|(view, load)| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        })
    }
}

/// Records this frame's draws and the feature targets omitted by the world passes.
#[derive(Default)]
struct FrameDrawStats {
    omitted_transparency_targets: translucent_targets::OmittedTransparencyTargets,
    atlas_animation_draw_calls: u64,
    opaque_draw_calls: u64,
    cutout_draw_calls: u64,
    translucent_draw_calls: u64,
    block_destroy_overlay_draw_calls: u64,
    sky_draw_calls: u64,
    entity_model_draw_calls: u64,
    debug_filled_box_draw_calls: u64,
    debug_text_draw_calls: u64,
    debug_line_draw_calls: u64,
    outline_composite_draw_calls: u64,
    transparency_combine_draw_calls: u64,
    transparency_blit_draw_calls: u64,
    particle_draw_calls: u64,
    weather_draw_calls: u64,
    item_entity_draw_calls: u64,
    item_model_draw_calls: u64,
    panorama_draw_calls: u64,
    selection_draw_calls: u64,
    chunk_border_draw_calls: u64,
    entity_scene_draw_calls: u64,
    hud_draw_calls: u64,
    screen_blur_draw_calls: u64,
    post_effect_draw_calls: u64,
    post_effect_depth_copy_draw_calls: u64,
    lightmap_draw_calls: u64,
    pipeline_switches: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameTransparencyCombine {
    None,
    VanillaDirect,
    GenericGraph,
}

/// Frame-wide pass activation and the selected transparency graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FramePlan {
    improved_transparency: bool,
    update_lightmap: bool,
    item_entity_depth_work: bool,
    particle_depth_work: bool,
    transparency_combine: FrameTransparencyCombine,
}

impl FramePlan {
    fn new(
        route: crate::LevelFrameRoute,
        update_lightmap: bool,
        item_entity_depth_work: bool,
        particle_depth_work: bool,
        post_effect_graphs_present: bool,
        vanilla_transparency_direct: bool,
    ) -> Self {
        debug_assert!(!vanilla_transparency_direct || post_effect_graphs_present);
        // Vanilla `LevelRenderer.getTransparencyChain`: without the transparency post chain the
        // frame renders like the regular route.
        let improved_transparency =
            route.uses_improved_transparency() && post_effect_graphs_present;
        let transparency_combine = if !improved_transparency {
            FrameTransparencyCombine::None
        } else if vanilla_transparency_direct {
            FrameTransparencyCombine::VanillaDirect
        } else {
            FrameTransparencyCombine::GenericGraph
        };
        Self {
            improved_transparency,
            update_lightmap,
            item_entity_depth_work,
            particle_depth_work,
            transparency_combine,
        }
    }
}

impl Renderer {
    pub fn render(&mut self, screenshot: Option<&Path>) -> Result<crate::RenderOutcome> {
        self.render_with_capture(screenshot, false)
    }

    pub fn render_with_capture(
        &mut self,
        screenshot: Option<&Path>,
        capture: bool,
    ) -> Result<crate::RenderOutcome> {
        let result = self.render_frame(screenshot, capture);
        // A skipped surface or a failed later pass must retire this frame's terrain commands.
        if !matches!(result, Ok(crate::RenderOutcome::Presented { .. })) {
            if let Some(finisher) = &mut self.terrain_finish {
                drop(finisher.complete());
            }
            self.restore_unsubmitted_frame_uploads();
        }
        result
    }

    fn restore_unsubmitted_frame_uploads(&mut self) {
        self.frame_uploads.discard();
        self.terrain.restore_unsubmitted_draws();
        self.pack.item_model_residency.restore_unsubmitted();
        self.screen_effect_projection = None;
        self.hud_geometry.invalidate();
        if let Some(texture) = self.pack.hud_vignette_texture.as_mut() {
            texture.invalidate_color();
        }
        if let Some(texture) = self.pack.hud_win_credits_vignette.as_mut() {
            texture.invalidate_color();
        }
        self.sky.restore_unsubmitted(
            &self.device,
            &mut self.frame_uploads,
            self.pack.celestial_atlas.as_ref(),
        );
        self.pack.entity_flames.restore_unsubmitted();
        self.pack.entity_shadows.restore_unsubmitted();
        self.pack.entity_name_tags.restore_unsubmitted();
        if self.entity_meshes_staged_unsubmitted {
            self.entity_meshes_staged_unsubmitted = false;
            self.pack
                .entity_geometry
                .restore_unsubmitted(&self.device, &mut self.frame_uploads);
            self.pack.first_person_player_arm_mesh = None;
            self.pack.first_person_dynamic_player_arm_mesh = None;
            self.pack.entity_meshes = Default::default();
            self.rebuild_first_person_player_arm_meshes();
            self.refresh_entity_model_meshes();
        } else {
            for lane in [
                EntityMeshLane::Solid,
                EntityMeshLane::Textured,
                EntityMeshLane::TexturedCull,
            ] {
                if let Some(binding) = self.pack.entity_meshes[lane]
                    .as_mut()
                    .and_then(|gpu| gpu.vertex_pose.as_ref())
                {
                    binding.stage_all(&self.device, &mut self.frame_uploads);
                }
            }
        }
    }

    fn render_frame(
        &mut self,
        screenshot: Option<&Path>,
        capture: bool,
    ) -> Result<crate::RenderOutcome> {
        let render_started_at = bbb_client_clock::now();
        let glint_elapsed_millis = render_started_at
            .saturating_duration_since(self.started_at)
            .as_secs_f64()
            * 1000.0;
        let frame_interval =
            render_started_at.saturating_duration_since(self.last_frame_started_at);
        self.last_frame_started_at = render_started_at;
        let gpu_async = self
            .gpu_timing
            .as_mut()
            .map_or(std::time::Duration::ZERO, |timing| {
                timing.poll(&self.device)
            });
        self.last_frame_profile = crate::renderer::RendererFrameProfile {
            frame_interval,
            gpu_async,
            gpu_async_poll: gpu_async,
            ..crate::renderer::RendererFrameProfile::default()
        };
        #[cfg(feature = "lifecycle-soak")]
        let injected_fault = self.lifecycle_soak_surface_faults.pop_front();
        #[cfg(not(feature = "lifecycle-soak"))]
        let injected_fault = None;
        let render_encoding_started_at = bbb_client_clock::now();
        // These features append copies to the shared upload encoder consumed by this frame.
        self.prepare_entity_flames();
        self.prepare_entity_shadows();
        let mut encoder = self.frame_uploads.take_encoder(&self.device);
        // Flush pending dynamic font atlas resizes before staging world text so name tags,
        // signs, text displays, and map labels all sample the current texture layout.
        self.flush_dynamic_font_atlas();
        if self.pack.item_frame_map_text_font_atlas.is_some() {
            self.prepare_entity_name_tags(&mut encoder);
            if self.flush_dynamic_font_atlas() {
                self.prepare_entity_name_tags(&mut encoder);
            }
        }
        self.pack
            .map_surface_residency
            .flush_pending_text_atlas_rescale(&self.device, &self.queue);
        self.stage_pending_camera_uniforms(&mut encoder);
        let (frame_plan, mut particle_batches) = self.prepare_frame_plan();
        if frame_plan.improved_transparency && self.terrain_finish.is_none() {
            self.terrain_finish = Some(crate::frame_flight::TerrainCommandFinisher::new()?);
        }
        let gpu_frame = self
            .gpu_timing
            .as_mut()
            .and_then(|timing| timing.begin_frame(&mut encoder));
        self.terrain
            .prepare_draws(&self.device, &mut encoder, self.frame_uploads.belt());
        self.pack.item_model_residency.stage_pending_uploads(
            &self.device,
            &mut encoder,
            self.frame_uploads.belt(),
        );
        let mut stats = FrameDrawStats::default();
        let terrain_animation_tick = self.terrain_atlas_animation_pass(&mut encoder, &mut stats);
        if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
            timing.mark_stage(&mut encoder, token, GpuFrameStage::TerrainAtlasAnimationEnd);
        }
        if frame_plan.update_lightmap {
            crate::lightmap::stage_lightmap_uniform(
                self.frame_uploads.belt(),
                &mut encoder,
                &self.lightmap.uniform_buffer,
                self.lightmap_environment,
            );
            self.lightmap_pass(&mut encoder, &mut stats);
        }
        let overlays_sample_main = self.post_effect_graphs.is_some()
            && (self.camera_post_effect_active || self.hud_screen_blur);
        #[cfg(target_arch = "wasm32")]
        let web_screenshot_pending = self.pending_web_screenshot.is_some();
        #[cfg(not(target_arch = "wasm32"))]
        let web_screenshot_pending = false;
        let direct = frame_plan.transparency_combine != FrameTransparencyCombine::GenericGraph
            && !overlays_sample_main
            && screenshot.is_none()
            && !capture
            && !web_screenshot_pending;
        let world_on_surface = direct && !frame_plan.improved_transparency;
        let has_hand = self.has_first_person_items();
        let combine_hand = has_hand
            && direct
            && frame_plan.transparency_combine == FrameTransparencyCombine::VanillaDirect
            && !gpu_frame.is_some_and(|token| token.detailed_stages)
            && self.debug_text_billboards.is_empty()
            && self.game_test_highlight_text_billboards.is_empty();
        self.ensure_frame_targets(
            frame_plan.improved_transparency,
            self.has_entity_outline_content() || combine_hand,
        );
        let encode_overlays = |renderer: &mut Self,
                               frame: &mut FrameColor<'_>,
                               stats: &mut FrameDrawStats|
         -> Result<()> {
            if !combine_hand {
                renderer.transparency_combine_pass(frame, stats, &frame_plan, direct)?;
                if let (Some(timing), Some(token)) = (renderer.gpu_timing.as_mut(), gpu_frame) {
                    timing.mark_stage(frame.encoder, token, GpuFrameStage::TransparencyCombineEnd);
                }
                renderer.debug_always_on_top_text_pass(frame, stats);
            }
            if has_hand {
                renderer.first_person_item_pass(frame, stats, combine_hand);
            }
            if renderer.post_effect_graphs.is_some() && renderer.has_entity_outline_content() {
                renderer.entity_outline_composite_pass(frame, stats);
            }
            let mut presentation = renderer.prepare_presentation_prefix(frame.encoder);
            renderer.camera_post_effect_pass(frame, stats, &mut presentation)?;
            renderer.entity_preview_pip_passes(
                frame,
                stats,
                glint_elapsed_millis,
                &mut presentation,
            );
            renderer.prepare_vignette(frame.encoder, &mut presentation);
            renderer.win_screen_portal_pass(frame, stats, &mut presentation);
            renderer.panorama_pass(frame, stats, &mut presentation);
            renderer.hud_passes(frame, stats, &mut presentation)?;
            renderer.win_screen_vignette_pass(frame, stats);
            Ok(())
        };
        if !world_on_surface {
            let view = self.targets.main.view.clone();
            let mut color = FrameColor {
                encoder: &mut encoder,
                view: &view,
            };
            self.render_world_passes(
                &mut color,
                &mut stats,
                &frame_plan,
                &particle_batches,
                gpu_frame,
            )?;
            if !direct {
                encode_overlays(self, &mut color, &mut stats)?;
            }
        }
        let cpu_encode_before_wait = if self.frame_phase_profiling_enabled {
            render_encoding_started_at.elapsed()
        } else {
            Default::default()
        };
        let acquired = crate::frame_flight::acquire_frame(
            &mut self.surface,
            &self.device,
            &self.config,
            &mut self.counters,
            injected_fault,
            render_encoding_started_at,
        );
        let previous_submission_poll = acquired.previous_submission_poll;
        let previous_submission_blocking_wait = acquired.previous_submission_blocking_wait;
        let previous_submission_wait = acquired.previous_submission_wait;
        let render_duration_before_surface = acquired.render_duration_before_surface;
        self.last_frame_profile.surface_acquire = acquired.surface_acquire;
        self.last_frame_profile
            .retired_submission_render_to_callback = acquired.retired_submission_render_to_callback;
        self.last_frame_profile
            .retired_submission_submit_to_callback = acquired.retired_submission_submit_to_callback;
        self.last_frame_profile
            .retired_submission_callback_to_wait_return =
            acquired.retired_submission_callback_to_wait_return;
        self.last_frame_profile.completion_observation_expected =
            acquired.completion_observation_expected;
        self.last_frame_profile.completion_observation_missing =
            acquired.completion_observation_missing;
        self.last_frame_profile.completion_observations_dropped =
            acquired.completion_observations_dropped;
        let frame = match acquired.frame {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                if let Some(animation) = self.pack.terrain_atlas_animation.as_mut() {
                    animation.restore_unsubmitted_tick(terrain_animation_tick);
                }
                if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
                    timing.cancel(token);
                }
                return Ok(crate::RenderOutcome::Skipped);
            }
            Err(error) => {
                if let Some(animation) = self.pack.terrain_atlas_animation.as_mut() {
                    animation.restore_unsubmitted_tick(terrain_animation_tick);
                }
                if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
                    timing.cancel(token);
                }
                return Err(error);
            }
        };
        let render_encoding_resumed_at = bbb_client_clock::now();
        if direct {
            let view = frame.texture().create_view(&Default::default());
            let mut color = FrameColor {
                encoder: &mut encoder,
                view: &view,
            };
            if world_on_surface {
                self.render_world_passes(
                    &mut color,
                    &mut stats,
                    &frame_plan,
                    &particle_batches,
                    gpu_frame,
                )?;
            }
            encode_overlays(self, &mut color, &mut stats)?;
        }
        particle_batches.opaque.clear();
        particle_batches.translucent.clear();
        self.particle_frame.quads = particle_batches;
        let presentation_tail_started_at = bbb_client_clock::now();
        if !direct {
            self.transparency_blit_pass(&frame, &mut encoder, &mut stats);
        }
        if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
            timing.mark_stage(&mut encoder, token, GpuFrameStage::PresentationTailEnd);
        }
        if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
            timing.finish_encoding(&mut encoder, token);
        }
        let presentation_tail_encode = presentation_tail_started_at.elapsed();
        let render_duration = render_duration_before_surface + render_encoding_resumed_at.elapsed();
        // Ensure any lazy font glyphs realized during world/debug text passes are uploaded
        // to the GPU before submitting this frame's command buffer.
        self.flush_dynamic_font_atlas();
        let result = self.finish_frame(
            encoder,
            frame,
            screenshot,
            capture,
            stats,
            gpu_frame,
            frame_interval,
            render_duration,
            cpu_encode_before_wait,
            previous_submission_poll,
            previous_submission_blocking_wait,
            previous_submission_wait,
            presentation_tail_encode,
            render_started_at,
        );
        if result.is_ok() && frame_plan.update_lightmap {
            self.lightmap_dirty = false;
        }
        result
    }

    fn prepare_frame_plan(&mut self) -> (FramePlan, ParticleBatches) {
        let particle_batches = std::mem::take(&mut self.particle_frame.quads);
        let post_effect_graphs_present = self.post_effect_graphs.is_some();
        let vanilla_transparency_direct = self
            .post_effect_graphs
            .as_ref()
            .is_some_and(|graphs| graphs.uses_vanilla_transparency_direct_path());
        let frame_plan = FramePlan::new(
            self.level_frame_route,
            self.lightmap_dirty,
            self.item_entity_target_has_depth_work(),
            self.particle_target_has_depth_work(&particle_batches),
            post_effect_graphs_present,
            vanilla_transparency_direct,
        );
        (frame_plan, particle_batches)
    }

    fn terrain_atlas_animation_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        stats: &mut FrameDrawStats,
    ) -> Option<(u64, u64)> {
        if let Some(animation) = self.pack.terrain_atlas_animation.as_mut() {
            let (draw_calls, tick) =
                animation.render(encoder, self.frame_uploads.belt(), &self.pack.terrain_atlas);
            stats.atlas_animation_draw_calls += draw_calls;
            tick
        } else {
            None
        }
    }

    fn lightmap_pass(&self, encoder: &mut wgpu::CommandEncoder, stats: &mut FrameDrawStats) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(LIGHTMAP_PASS_LABEL),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.lightmap.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipelines.lightmap);
            stats.pipeline_switches += 1;
            pass.set_bind_group(0, &self.lightmap.bind_group, &[]);
            pass.draw(0..3, 0..1);
            stats.lightmap_draw_calls += 1;
        }
    }

    fn debug_always_on_top_text_pass(
        &mut self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
    ) {
        let Some(camera_pose) = self.camera_pose else {
            return;
        };
        if self.pack.item_frame_map_text_font_atlas.is_none() {
            return;
        }
        let mut mesh = build_debug_text_mesh_for_layer(
            self.debug_text_billboards
                .iter()
                .chain(self.game_test_highlight_text_billboards.iter()),
            &self.pack.hud_font_glyphs,
            camera_pose,
            true,
        );
        // Lazy glyphs realized for always-on-top debug text may upload subrects or grow
        // the dynamic font atlas; flush before binding so the pass samples the live layout.
        if self.flush_dynamic_font_atlas() {
            mesh = build_debug_text_mesh_for_layer(
                self.debug_text_billboards
                    .iter()
                    .chain(self.game_test_highlight_text_billboards.iter()),
                &self.pack.hud_font_glyphs,
                camera_pose,
                true,
            );
        }
        let Some(font_atlas) = self.pack.item_frame_map_text_font_atlas.as_ref() else {
            return;
        };
        let vertex_count = mesh.vertices.len();
        let index_count = mesh.indices.len();
        let has_vertices = self.frame_debug_always_on_top_text_vertices.upload_staged(
            &self.device,
            frame.encoder,
            self.frame_uploads.belt(),
            bytemuck::cast_slice(&mesh.vertices),
        );
        let has_indices = self.frame_debug_always_on_top_text_indices.upload_staged(
            &self.device,
            frame.encoder,
            self.frame_uploads.belt(),
            bytemuck::cast_slice(&mesh.indices),
        );
        if !has_vertices || !has_indices {
            debug_assert_eq!(vertex_count, 0);
            debug_assert_eq!(index_count, 0);
            return;
        }

        let vertex_buffer = self
            .frame_debug_always_on_top_text_vertices
            .buffer()
            .expect("always-on-top debug text vertices uploaded");
        let index_buffer = self
            .frame_debug_always_on_top_text_indices
            .buffer()
            .expect("always-on-top debug text indices uploaded");
        let mut pass = frame.begin_pass(
            DEBUG_ALWAYS_ON_TOP_TEXT_PASS_LABEL,
            wgpu::LoadOp::Load,
            Some((&self.targets.depth.view, wgpu::LoadOp::Clear(1.0))),
        );
        pass.set_pipeline(&self.pipelines.debug_text);
        stats.pipeline_switches += 1;
        pass.set_bind_group(0, &font_atlas.bind_group, &[]);
        pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..index_count as u32, 0, 0..1);
        stats.debug_text_draw_calls += 1;
    }

    fn has_first_person_items(&self) -> bool {
        !self.hide_gui
            && (self.pack.first_person_player_arm_mesh.is_some()
                || self.pack.first_person_dynamic_player_arm_mesh.is_some()
                || self
                    .has_resident_map_surfaces(crate::item_models::MapSurfaceTarget::FirstPerson)
                || self.has_resident_map_text(crate::item_models::MapSurfaceTarget::FirstPerson)
                || self
                    .pack
                    .item_model_residency
                    .has_draws_for_target(crate::item_models::ItemModelDrawTarget::FirstPerson))
    }

    fn first_person_item_pass(
        &self,
        frame: &mut FrameColor<'_>,
        stats: &mut FrameDrawStats,
        combine: bool,
    ) {
        // The combine samples world depth. Outline depth is attachment-only and its world pass
        // is complete, so it can provide the hands' independent depth clear.
        let (load, depth) = if combine {
            (
                wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                &self.targets.entity_outline().depth.view,
            )
        } else {
            (wgpu::LoadOp::Load, &self.targets.depth.view)
        };
        let mut pass = frame.begin_pass(
            FIRST_PERSON_ITEM_PASS_LABEL,
            load,
            Some((depth, wgpu::LoadOp::Clear(1.0))),
        );
        if combine {
            self.record_transparency_combine(&mut pass, &self.transparency_hand_pipeline, stats);
        }
        let lightmap = Some(&self.lightmap.sample_bind_group);
        draw_mesh_lanes(
            &mut pass,
            &[
                lane(
                    &self.pack.first_person_player_arm_mesh,
                    &self.pipelines.entity_model_translucent,
                    self.pack
                        .entity_model_texture_atlas
                        .as_ref()
                        .map(|atlas| &atlas.hud3d_bind_group),
                    lightmap,
                ),
                lane(
                    &self.pack.first_person_dynamic_player_arm_mesh,
                    &self.pipelines.entity_model_translucent,
                    self.entity_dynamic_player_skin_atlas
                        .as_ref()
                        .map(|atlas| &atlas.hud3d_bind_group),
                    lightmap,
                ),
            ],
            &mut stats.pipeline_switches,
            &mut stats.entity_model_draw_calls,
        );

        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
            &[crate::item_models::ItemModelDrawPhase::Solid],
            Some(&[crate::item_models::ItemModelAtlasKind::Blocks]),
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
            &[crate::item_models::ItemModelDrawPhase::Solid],
            Some(&[crate::item_models::ItemModelAtlasKind::Items]),
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
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
        self.draw_resident_map_backgrounds(&mut pass, stats);
        self.draw_resident_dynamic_maps(
            &mut pass,
            crate::item_models::MapSurfaceTarget::FirstPerson,
            stats,
        );
        self.draw_resident_map_decorations(
            &mut pass,
            crate::item_models::MapSurfaceTarget::FirstPerson,
            stats,
        );
        self.draw_resident_map_text(
            &mut pass,
            crate::item_models::MapSurfaceTarget::FirstPerson,
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
            &[crate::item_models::ItemModelDrawPhase::Glint],
            None,
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
            &[crate::item_models::ItemModelDrawPhase::Translucent],
            Some(&[crate::item_models::ItemModelAtlasKind::Blocks]),
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
            &[crate::item_models::ItemModelDrawPhase::Translucent],
            Some(&[crate::item_models::ItemModelAtlasKind::Items]),
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
            &[
                crate::item_models::ItemModelDrawPhase::Translucent,
                crate::item_models::ItemModelDrawPhase::TranslucentNoCull,
            ],
            Some(RESIDENT_SPECIAL_ITEM_ATLASES),
            stats,
        );
        self.draw_resident_item_models(
            &mut pass,
            crate::item_models::ItemModelDrawTarget::FirstPerson,
            &[crate::item_models::ItemModelDrawPhase::GlintTranslucent],
            None,
            stats,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_frame(
        &mut self,
        mut encoder: wgpu::CommandEncoder,
        frame: FrameTarget,
        screenshot: Option<&Path>,
        capture: bool,
        stats: FrameDrawStats,
        gpu_frame: Option<crate::gpu_timing::GpuFrameToken>,
        frame_interval: std::time::Duration,
        render_duration: std::time::Duration,
        cpu_encode_before_wait: std::time::Duration,
        previous_submission_poll: std::time::Duration,
        previous_submission_blocking_wait: std::time::Duration,
        previous_submission_wait: std::time::Duration,
        presentation_tail_encode: std::time::Duration,
        render_started_at: bbb_client_clock::Instant,
    ) -> Result<crate::RenderOutcome> {
        #[cfg(target_arch = "wasm32")]
        let web_screenshot_cb = self.pending_web_screenshot.take();
        #[cfg(not(target_arch = "wasm32"))]
        let need_web_screenshot = false;
        #[cfg(target_arch = "wasm32")]
        let need_web_screenshot = web_screenshot_cb.is_some();

        // Capture the final color target; some surfaces only support rendering.
        let readback = match screenshot.is_some() || capture || need_web_screenshot {
            true => match self.prepare_screenshot_copy(&mut encoder, &self.targets.main._texture) {
                Ok(readback) => Some(readback),
                Err(error) => {
                    #[cfg(target_arch = "wasm32")]
                    if let Some(cb) = web_screenshot_cb {
                        cb(Err(anyhow::anyhow!("{error:#}")));
                    }
                    if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
                        timing.cancel(token);
                    }
                    return Err(error);
                }
            },
            false => None,
        };

        let finish_started_at = bbb_client_clock::now();
        debug_assert!(
            !self.frame_uploads.has_pending_encoder(),
            "staged uploads must record into the active frame CommandEncoder after take_encoder"
        );
        self.frame_uploads.finish_belt();
        let commands = encoder.finish();
        let tail_finish = finish_started_at.elapsed();
        let (head, head_finish) = self
            .terrain_finish
            .as_mut()
            .and_then(crate::frame_flight::TerrainCommandFinisher::complete)
            .unzip();
        let finish_wall = if head.is_some() {
            finish_started_at.elapsed()
        } else {
            tail_finish
        };
        self.frame_uploads.flush_before_external_submit(&self.queue);
        let submission = crate::frame_flight::submit_and_present(
            &self.queue,
            head.into_iter().chain(std::iter::once(commands)),
            finish_wall,
            frame,
            self.submission_profiling_enabled,
        );
        self.frame_uploads.recall_belt();
        self.terrain.mark_draws_submitted();
        self.pack.item_model_residency.mark_submitted();
        self.pack
            .item_model_residency
            .drain_retired_into(self.frame_uploads.belt());
        self.frame_uploads.retire_after_external_submit(&self.queue);
        self.entity_meshes_staged_unsubmitted = false;
        self.pending_camera_uniforms.set(None);
        self.pending_cloud_uniform.set(None);
        let present_duration = submission.queue_submit;
        let swap_buffers_duration = submission.swap_buffers_present;
        self.last_frame_profile.render = render_duration;
        self.last_frame_profile.present = present_duration;
        self.last_frame_profile.swap_buffers = swap_buffers_duration;
        self.last_frame_profile.cpu_encode_before_wait = cpu_encode_before_wait;
        self.last_frame_profile.previous_submission_poll = previous_submission_poll;
        self.last_frame_profile.previous_submission_blocking_wait =
            previous_submission_blocking_wait;
        self.last_frame_profile.previous_submission_wait = previous_submission_wait;
        self.last_frame_profile.presentation_tail_encode = presentation_tail_encode;
        self.last_frame_profile.queue_submit = present_duration;
        if self.submission_profiling_enabled {
            self.last_frame_profile.command_encoder_finish =
                tail_finish + head_finish.unwrap_or_default();
            self.last_frame_profile.world_command_finish_wait = finish_wall - tail_finish;
        }
        self.last_frame_profile.queue_submit_api = submission.queue_submit_api;
        self.last_frame_profile.swap_buffers_present = swap_buffers_duration;
        self.last_frame_profile.post_present_fence_submit = submission.post_present_fence_submit;
        self.last_frame_profile.in_flight_depth_at_submit = submission.in_flight_depth_at_submit;
        self.last_frame_profile.input_latency = self.input_latency.complete(
            submission.queue_submitted_at,
            submission.present_call_returned_at,
        );
        if let (Some(timing), Some(token)) = (self.gpu_timing.as_mut(), gpu_frame) {
            timing.after_submit(token, frame_interval);
        }

        #[cfg(target_arch = "wasm32")]
        let readback = match (readback, web_screenshot_cb) {
            (Some(pending), Some(cb)) => {
                Self::spawn_web_screenshot_readback(pending, cb);
                self.counters.screenshots_written += 1;
                None
            }
            (other, _) => other,
        };

        let pixels = readback.map(|readback| self.read_screenshot_pixels(readback));
        let screenshot = screenshot.map(|path| match pixels.as_ref().unwrap() {
            Ok(pixels) => Self::save_screenshot_pixels(pixels, path),
            Err(error) => Err(anyhow::anyhow!("{error:#}")),
        });
        let capture = if capture { pixels } else { None };
        if screenshot.as_ref().is_some_and(Result::is_ok) {
            self.counters.screenshots_written += 1;
        }

        self.counters.frame_index += 1;
        let resident_upload = self.pack.item_model_residency.take_upload_observation();
        self.counters.item_model_instances = resident_upload.instances;
        self.counters.item_model_vertex_upload_bytes = resident_upload.vertex_bytes;
        self.counters.item_model_index_upload_bytes = resident_upload.index_bytes;
        self.counters.item_instance_upload_bytes = resident_upload.instance_bytes;
        self.counters.item_instance_upload_writes = resident_upload.instance_writes;
        self.counters.item_instance_buffer_replacements =
            resident_upload.instance_buffer_replacements;
        self.counters.opaque_draw_calls = stats.opaque_draw_calls;
        self.counters.cutout_draw_calls = stats.cutout_draw_calls;
        self.counters.translucent_draw_calls = stats.translucent_draw_calls;
        self.counters.block_destroy_overlay_draw_calls = stats.block_destroy_overlay_draw_calls;
        self.counters.sky_draw_calls = stats.sky_draw_calls;
        self.counters.particle_draw_calls = stats.particle_draw_calls;
        self.counters.weather_draw_calls = stats.weather_draw_calls;
        self.counters.item_entity_draw_calls = stats.item_entity_draw_calls;
        self.counters.panorama_draw_calls = stats.panorama_draw_calls;
        self.counters.selection_draw_calls = stats.selection_draw_calls;
        self.counters.chunk_border_draw_calls = stats.chunk_border_draw_calls;
        self.counters.entity_scene_draw_calls =
            stats.entity_scene_draw_calls + stats.entity_model_draw_calls;
        self.counters.hud_draw_calls = stats.hud_draw_calls;
        self.counters.screen_blur_draw_calls = stats.screen_blur_draw_calls;
        self.counters.post_effect_draw_calls = stats.post_effect_draw_calls;
        self.counters.draw_calls = stats.atlas_animation_draw_calls
            + stats.opaque_draw_calls
            + stats.cutout_draw_calls
            + stats.translucent_draw_calls
            + stats.block_destroy_overlay_draw_calls
            + stats.sky_draw_calls
            + stats.entity_model_draw_calls
            + stats.debug_filled_box_draw_calls
            + stats.debug_text_draw_calls
            + stats.debug_line_draw_calls
            + stats.outline_composite_draw_calls
            + stats.transparency_combine_draw_calls
            + stats.transparency_blit_draw_calls
            + stats.particle_draw_calls
            + stats.weather_draw_calls
            + stats.item_entity_draw_calls
            + stats.panorama_draw_calls
            + stats.item_model_draw_calls
            + stats.selection_draw_calls
            + stats.chunk_border_draw_calls
            + stats.entity_scene_draw_calls
            + stats.hud_draw_calls
            + stats.screen_blur_draw_calls
            + stats.post_effect_draw_calls
            + stats.post_effect_depth_copy_draw_calls
            + stats.lightmap_draw_calls;
        self.counters.pipeline_switches = stats.pipeline_switches;
        if self.frame_phase_profiling_enabled {
            self.last_frame_profile.render_call_total = render_started_at.elapsed();
        }
        Ok(crate::RenderOutcome::Presented {
            screenshot,
            capture,
        })
    }

    fn draw_particle_batch<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        vertex_buffer: &'a wgpu::Buffer,
        batch: &'a ParticleBatch,
        stats: &mut FrameDrawStats,
    ) {
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        for draw in &batch.draws {
            let Some(bind_group) = self.particle_texture_atlas_bind_group(draw.texture_atlas)
            else {
                continue;
            };
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_bind_group(1, &self.lightmap.sample_bind_group, &[]);
            let instances = &draw.instances;
            if self.uses_instanced_particles() {
                pass.draw(0..6, instances.clone());
            } else {
                pass.draw(instances.start * 6..instances.end * 6, 0..1);
            }
            stats.particle_draw_calls += 1;
        }
    }

    fn particle_texture_atlas_bind_group(
        &self,
        atlas: ParticleTextureAtlasKind,
    ) -> Option<&wgpu::BindGroup> {
        match atlas {
            ParticleTextureAtlasKind::Particles => self
                .pack
                .particle_atlas
                .as_ref()
                .map(|atlas| &atlas.bind_group),
            ParticleTextureAtlasKind::Terrain => Some(&self.pack.terrain_bind_group),
            ParticleTextureAtlasKind::Items => self
                .pack
                .item_entity_atlas
                .as_ref()
                .map(|atlas| &atlas.bind_group),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{physical_scissor_from_gui, FramePlan, FrameTransparencyCombine, Renderer};
    use winit::dpi::PhysicalSize;

    mod armor_to_world;
    mod gui_projection;
}
