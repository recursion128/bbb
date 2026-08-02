use anyhow::Result;
use bbb_render_types::{CoreShaderSourceProjection, PostEffectResourceProjection};

use crate::{post_effect_graph::PostEffectGraphRuntime, Renderer};

pub(crate) type ResourceOwnerRestore = Box<dyn FnOnce(&mut Renderer) + Send>;

/// Previous renderer-derived resource owners retained while one reload is staged.
///
/// The renderer remains the sole owner. Reload upload methods install new handles
/// only while this checkpoint retains every accepted handle they can replace. A
/// failed stage restores those exact owners; no resource is re-decoded or
/// re-uploaded and no mixed generation can escape the synchronous render-thread
/// barrier.
pub(crate) struct RendererResourceReloadCheckpoint {
    pub(crate) restores: Vec<ResourceOwnerRestore>,
    previous_counters: crate::RendererCounters,
}

struct GlintCorePipelines {
    entity: wgpu::RenderPipeline,
    armor_entity: wgpu::RenderPipeline,
    dynamic_item: wgpu::RenderPipeline,
    resident_item: wgpu::RenderPipeline,
    hud_item: wgpu::RenderPipeline,
}

impl GlintCorePipelines {
    fn prepare(
        renderer: &Renderer,
        shaders: Option<&crate::core_shaders::AdaptedGlintShaders>,
    ) -> Result<Self> {
        Ok(Self {
            entity: crate::entity_models::create_entity_model_entity_glint_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                shaders,
            )?,
            armor_entity: crate::entity_models::create_entity_model_armor_entity_glint_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                shaders,
            )?,
            dynamic_item: crate::item_models::create_item_model_glint_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                shaders,
            )?,
            resident_item: crate::item_models::create_resident_item_model_glint_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                shaders,
            )?,
            hud_item: crate::hud::create_hud_item_glint_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                &renderer.hud_bind_group_layout,
                shaders,
            )?,
        })
    }
}

struct EntityShaderVariantPipelines {
    scroll: wgpu::RenderPipeline,
    scroll_additive: wgpu::RenderPipeline,
    guardian_beam: wgpu::RenderPipeline,
    end_crystal_beam: wgpu::RenderPipeline,
    dynamic_item_z_offset_forward: wgpu::RenderPipeline,
    resident_item_z_offset_forward: wgpu::RenderPipeline,
    resident_item_cutout_no_cull: wgpu::RenderPipeline,
    resident_item_cutout_z_offset: wgpu::RenderPipeline,
    resident_item_pattern: wgpu::RenderPipeline,
    resident_item_translucent_no_cull: wgpu::RenderPipeline,
}

impl EntityShaderVariantPipelines {
    fn prepare(
        renderer: &Renderer,
        shaders: Option<&crate::core_shaders::AdaptedEntityShaders>,
    ) -> Result<Self> {
        Ok(Self {
            scroll: crate::entity_models::create_entity_model_scroll_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                &renderer.lightmap_sample_bind_group_layout,
                shaders,
            )?,
            scroll_additive: crate::entity_models::create_entity_model_scroll_additive_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                shaders,
            )?,
            guardian_beam: crate::entity_models::create_entity_model_guardian_beam_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                &renderer.lightmap_sample_bind_group_layout,
                shaders,
            )?,
            end_crystal_beam: crate::entity_models::create_entity_model_end_crystal_beam_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                &renderer.lightmap_sample_bind_group_layout,
                shaders,
            )?,
            dynamic_item_z_offset_forward:
                crate::item_models::create_item_model_z_offset_forward_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    shaders,
                )?,
            resident_item_z_offset_forward:
                crate::item_models::create_resident_item_model_z_offset_forward_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    shaders,
                )?,
            resident_item_cutout_no_cull:
                crate::item_models::create_resident_item_model_cutout_no_cull_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    shaders,
                )?,
            resident_item_cutout_z_offset:
                crate::item_models::create_resident_item_model_cutout_z_offset_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    shaders,
                )?,
            resident_item_pattern: crate::item_models::create_resident_item_model_pattern_pipeline(
                &renderer.device,
                renderer.config.format,
                &renderer.terrain_bind_group_layout,
                &renderer.lightmap_sample_bind_group_layout,
                shaders,
            )?,
            resident_item_translucent_no_cull:
                crate::item_models::create_resident_item_model_translucent_no_cull_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    shaders,
                )?,
        })
    }
}

/// One fully prepared set of core-shader GPU owners.
///
/// This is the only generation-shaped owner for core pipelines. Family values
/// group homogeneous handles but do not publish an independent generation.
/// Swapping this value with the renderer is the single field mapping used for
/// both commit and rollback.
struct CorePipelineGeneration {
    generation: Option<u64>,
    terrain: crate::gpu::TerrainCorePipelines,
    entity: Option<crate::entity_models::EntityCorePipelines>,
    entity_variants: Option<EntityShaderVariantPipelines>,
    item: Option<crate::item_models::ItemCorePipelines>,
    particle: Option<crate::particles::ParticleCorePipelines>,
    weather: Option<wgpu::RenderPipeline>,
    debug_filled: Option<wgpu::RenderPipeline>,
    sunrise: Option<wgpu::RenderPipeline>,
    glint: Option<GlintCorePipelines>,
}

impl CorePipelineGeneration {
    fn prepare(
        renderer: &Renderer,
        resources: &CoreShaderSourceProjection,
        allow_embedded_fallback: bool,
    ) -> Result<Self> {
        let terrain = crate::gpu::create_terrain_core_pipelines(
            &renderer.device,
            renderer.config.format,
            &renderer.terrain_bind_group_layout,
            &renderer.lightmap_sample_bind_group_layout,
            resources,
            renderer.terrain_texture_filtering,
            allow_embedded_fallback,
        )?;
        let startup_without_sources = allow_embedded_fallback && resources.source_count() == 0;
        let entity_shaders = (!startup_without_sources)
            .then(|| crate::core_shaders::adapt_selected_entity_shaders(resources))
            .transpose()?
            .flatten();
        let entity = (!startup_without_sources)
            .then(|| {
                crate::entity_models::create_entity_core_pipelines(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    entity_shaders.as_ref(),
                )
            })
            .transpose()?;
        let entity_variants = (!startup_without_sources)
            .then(|| EntityShaderVariantPipelines::prepare(renderer, entity_shaders.as_ref()))
            .transpose()?;
        let item = (!startup_without_sources)
            .then(|| {
                crate::item_models::create_item_core_pipelines(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    resources,
                )
            })
            .transpose()?;
        let particle_shaders = (!startup_without_sources)
            .then(|| crate::core_shaders::adapt_selected_particle_shaders(resources))
            .transpose()?
            .flatten();
        let particle = (!startup_without_sources)
            .then(|| {
                crate::particles::create_particle_core_pipelines(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    particle_shaders.as_ref(),
                )
            })
            .transpose()?;
        let weather = (!startup_without_sources)
            .then(|| {
                crate::weather::create_weather_core_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    &renderer.lightmap_sample_bind_group_layout,
                    particle_shaders.as_ref(),
                )
            })
            .transpose()?;
        let position_color_shaders = (!startup_without_sources)
            .then(|| crate::core_shaders::adapt_selected_position_color_shaders(resources))
            .transpose()?
            .flatten();
        let debug_filled = (!startup_without_sources)
            .then(|| {
                crate::debug_filled_boxes::create_debug_filled_box_core_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    position_color_shaders.as_ref(),
                )
            })
            .transpose()?;
        let sunrise = (!startup_without_sources)
            .then(|| {
                crate::sky::create_sunrise_sunset_core_pipeline(
                    &renderer.device,
                    renderer.config.format,
                    &renderer.terrain_bind_group_layout,
                    position_color_shaders.as_ref(),
                )
            })
            .transpose()?;
        let glint_shaders = (!startup_without_sources)
            .then(|| crate::core_shaders::adapt_selected_glint_shaders(resources))
            .transpose()?
            .flatten();
        let glint = (!startup_without_sources)
            .then(|| GlintCorePipelines::prepare(renderer, glint_shaders.as_ref()))
            .transpose()?;
        Ok(Self {
            generation: Some(resources.generation()),
            terrain,
            entity,
            entity_variants,
            item,
            particle,
            weather,
            debug_filled,
            sunrise,
            glint,
        })
    }

    fn swap_with_renderer(&mut self, renderer: &mut Renderer) {
        std::mem::swap(
            &mut self.terrain.solid,
            &mut renderer.terrain_solid_pipeline,
        );
        std::mem::swap(
            &mut self.terrain.cutout,
            &mut renderer.terrain_cutout_pipeline,
        );
        std::mem::swap(
            &mut self.terrain.translucent,
            &mut renderer.terrain_translucent_pipeline,
        );
        if let Some(entity) = self.entity.as_mut() {
            std::mem::swap(
                &mut entity.textured,
                &mut renderer.entity_model_textured_pipeline,
            );
            std::mem::swap(
                &mut entity.textured_cull,
                &mut renderer.entity_model_textured_cull_pipeline,
            );
            std::mem::swap(&mut entity.solid, &mut renderer.entity_model_solid_pipeline);
            std::mem::swap(
                &mut entity.dissolve,
                &mut renderer.entity_model_dissolve_pipeline,
            );
            std::mem::swap(
                &mut entity.solid_z_offset_forward,
                &mut renderer.entity_model_solid_z_offset_forward_pipeline,
            );
            std::mem::swap(
                &mut entity.cutout_z_offset,
                &mut renderer.entity_model_cutout_z_offset_pipeline,
            );
            std::mem::swap(
                &mut entity.armor_cutout,
                &mut renderer.entity_model_armor_cutout_pipeline,
            );
            std::mem::swap(
                &mut entity.armor_decal,
                &mut renderer.entity_model_armor_decal_pipeline,
            );
            std::mem::swap(
                &mut entity.armor_translucent,
                &mut renderer.entity_model_armor_translucent_pipeline,
            );
            std::mem::swap(
                &mut entity.translucent,
                &mut renderer.entity_model_translucent_pipeline,
            );
            std::mem::swap(
                &mut entity.banner_pattern,
                &mut renderer.entity_model_banner_pattern_pipeline,
            );
            std::mem::swap(
                &mut entity.translucent_cull,
                &mut renderer.entity_model_translucent_cull_pipeline,
            );
            std::mem::swap(
                &mut entity.translucent_emissive,
                &mut renderer.entity_model_translucent_emissive_pipeline,
            );
            std::mem::swap(&mut entity.eyes, &mut renderer.entity_model_eyes_pipeline);
        }
        if let Some(entity) = self.entity_variants.as_mut() {
            std::mem::swap(
                &mut entity.scroll,
                &mut renderer.entity_model_scroll_pipeline,
            );
            std::mem::swap(
                &mut entity.scroll_additive,
                &mut renderer.entity_model_scroll_additive_pipeline,
            );
            std::mem::swap(
                &mut entity.guardian_beam,
                &mut renderer.entity_model_guardian_beam_pipeline,
            );
            std::mem::swap(
                &mut entity.end_crystal_beam,
                &mut renderer.entity_model_end_crystal_beam_pipeline,
            );
            std::mem::swap(
                &mut entity.dynamic_item_z_offset_forward,
                &mut renderer.item_model_z_offset_forward_pipeline,
            );
            std::mem::swap(
                &mut entity.resident_item_z_offset_forward,
                &mut renderer.resident_item_model_z_offset_forward_pipeline,
            );
            std::mem::swap(
                &mut entity.resident_item_cutout_no_cull,
                &mut renderer.resident_item_model_cutout_no_cull_pipeline,
            );
            std::mem::swap(
                &mut entity.resident_item_cutout_z_offset,
                &mut renderer.resident_item_model_cutout_z_offset_pipeline,
            );
            std::mem::swap(
                &mut entity.resident_item_pattern,
                &mut renderer.resident_item_model_pattern_pipeline,
            );
            std::mem::swap(
                &mut entity.resident_item_translucent_no_cull,
                &mut renderer.resident_item_model_translucent_no_cull_pipeline,
            );
        }
        if let Some(item) = self.item.as_mut() {
            std::mem::swap(&mut item.dynamic_cutout, &mut renderer.item_model_pipeline);
            std::mem::swap(
                &mut item.dynamic_translucent,
                &mut renderer.item_model_translucent_pipeline,
            );
            std::mem::swap(
                &mut item.resident_cutout,
                &mut renderer.resident_item_model_pipeline,
            );
            std::mem::swap(
                &mut item.resident_translucent,
                &mut renderer.resident_item_model_translucent_pipeline,
            );
        }
        if let Some(particle) = self.particle.as_mut() {
            std::mem::swap(&mut particle.opaque, &mut renderer.opaque_particle_pipeline);
            std::mem::swap(
                &mut particle.translucent,
                &mut renderer.translucent_particle_pipeline,
            );
        }
        if let Some(weather) = self.weather.as_mut() {
            std::mem::swap(weather, &mut renderer.weather_pipeline);
        }
        if let Some(debug_filled) = self.debug_filled.as_mut() {
            std::mem::swap(debug_filled, &mut renderer.debug_filled_box_pipeline);
        }
        if let Some(sunrise) = self.sunrise.as_mut() {
            std::mem::swap(sunrise, &mut renderer.sunrise_sunset_pipeline);
        }
        if let Some(glint) = self.glint.as_mut() {
            std::mem::swap(
                &mut glint.entity,
                &mut renderer.entity_model_entity_glint_pipeline,
            );
            std::mem::swap(
                &mut glint.armor_entity,
                &mut renderer.entity_model_armor_entity_glint_pipeline,
            );
            std::mem::swap(
                &mut glint.dynamic_item,
                &mut renderer.item_model_glint_pipeline,
            );
            std::mem::swap(
                &mut glint.resident_item,
                &mut renderer.resident_item_model_glint_pipeline,
            );
            std::mem::swap(&mut glint.hud_item, &mut renderer.hud_item_glint_pipeline);
        }
        std::mem::swap(&mut self.generation, &mut renderer.core_shader_generation);
    }
}

impl Renderer {
    pub fn reload_core_shaders(
        &mut self,
        resources: &CoreShaderSourceProjection,
        allow_embedded_fallback: bool,
    ) -> Result<()> {
        let mut next = crate::post_effect_graph::capture_gpu_errors(&self.device, || {
            CorePipelineGeneration::prepare(self, resources, allow_embedded_fallback)
        })?;
        next.swap_with_renderer(self);
        if self.resource_reload_in_progress() {
            self.register_resource_owner_restore(Box::new(move |renderer| {
                next.swap_with_renderer(renderer);
            }));
        }
        Ok(())
    }

    pub fn core_shader_generation(&self) -> Option<u64> {
        self.core_shader_generation
    }

    pub fn reload_post_effect_graphs(
        &mut self,
        resources: &PostEffectResourceProjection,
    ) -> Result<()> {
        let next = PostEffectGraphRuntime::new(
            &self.device,
            &self.queue,
            self.config.format,
            self.config.width,
            self.config.height,
            resources,
        )?;
        let previous = self.post_effect_graphs.replace(next);
        if self.resource_reload_in_progress() {
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.post_effect_graphs = previous;
            }));
        }
        Ok(())
    }

    pub fn post_effect_graph_generation(&self) -> Option<u64> {
        self.post_effect_graphs
            .as_ref()
            .map(PostEffectGraphRuntime::generation)
    }

    /// Runs all renderer resource uploads as one all-or-nothing render-thread
    /// transaction. The closure is synchronous: no frame can observe the empty
    /// staging slots or a partially installed generation.
    pub fn transactional_resource_reload<T>(
        &mut self,
        apply: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.begin_resource_reload();
        match apply(self) {
            Ok(value) => {
                self.resource_reload_checkpoint = None;
                Ok(value)
            }
            Err(error) => {
                self.rollback_resource_reload();
                Err(error)
            }
        }
    }

    pub(super) fn resource_reload_in_progress(&self) -> bool {
        self.resource_reload_checkpoint.is_some()
    }

    pub(super) fn register_resource_owner_restore(&mut self, restore: ResourceOwnerRestore) {
        self.resource_reload_checkpoint
            .as_mut()
            .expect("resource restore registered outside a reload transaction")
            .restores
            .push(restore);
    }

    fn begin_resource_reload(&mut self) {
        assert!(
            self.resource_reload_checkpoint.is_none(),
            "renderer resource reload transactions cannot nest"
        );
        let mut restores: Vec<ResourceOwnerRestore> = Vec::new();

        macro_rules! take_owner {
            ($field:ident) => {{
                let previous = std::mem::take(&mut self.$field);
                restores.push(Box::new(move |renderer: &mut Renderer| {
                    renderer.$field = previous;
                }));
            }};
        }

        macro_rules! clone_owner {
            ($field:ident) => {{
                let previous = self.$field.clone();
                restores.push(Box::new(move |renderer: &mut Renderer| {
                    renderer.$field = previous;
                }));
            }};
        }

        // Particle/item/terrain lookup owners. The mandatory terrain atlas and
        // bind groups are captured lazily after their replacements have been
        // built successfully (see `upload_terrain_texture_atlas_mips`).
        take_owner!(particle_atlas);
        take_owner!(terrain_particle_sprite_uvs);
        take_owner!(terrain_particle_translucent_sprites);
        take_owner!(item_particle_sprite_uvs);
        take_owner!(item_particle_translucent_sprites);
        take_owner!(item_entity_atlas);
        take_owner!(item_glint_texture);
        take_owner!(armor_glint_texture);
        take_owner!(item_frame_map_text_font_atlas);
        take_owner!(first_person_map_background_atlas);
        take_owner!(item_frame_map_atlas);
        take_owner!(item_frame_map_decoration_atlas);
        let empty_map_surface_residency = crate::item_models::MapSurfaceResidency::new(
            &self.device,
            self.config.format,
            &self.terrain_bind_group_layout,
            &self.lightmap_sample_bind_group_layout,
        );
        let previous =
            std::mem::replace(&mut self.map_surface_residency, empty_map_surface_residency);
        restores.push(Box::new(move |renderer: &mut Renderer| {
            renderer.map_surface_residency = previous;
        }));

        // Static entity atlas plus every mesh derived from it. Dynamic image
        // owners and canonical instances stay live and are used to rebuild the
        // staged generation.
        take_owner!(entity_model_texture_atlas);
        take_owner!(entity_model_mesh);
        take_owner!(entity_model_water_mask_mesh);
        take_owner!(entity_model_solid_mesh);
        take_owner!(entity_model_textured_mesh);
        take_owner!(entity_model_textured_cull_mesh);
        take_owner!(entity_model_dissolve_mesh);
        take_owner!(entity_model_solid_z_offset_forward_mesh);
        take_owner!(entity_model_cutout_z_offset_mesh);
        take_owner!(entity_model_armor_cutout_mesh);
        take_owner!(entity_model_armor_trim_mesh);
        take_owner!(entity_model_armor_trim_decal_mesh);
        take_owner!(entity_model_translucent_mesh);
        take_owner!(entity_model_armor_translucent_mesh);
        take_owner!(entity_model_translucent_emissive_mesh);
        take_owner!(entity_model_item_entity_translucent_mesh);
        take_owner!(entity_model_item_entity_translucent_cull_mesh);
        take_owner!(entity_model_opaque_beam_draws);
        take_owner!(entity_model_sorted_main_translucent_draws);
        take_owner!(entity_model_sorted_translucent_draws);
        take_owner!(entity_shadow_texture);
        take_owner!(entity_model_sorted_item_entity_draws);
        take_owner!(entity_model_eyes_mesh);
        take_owner!(entity_model_dragon_rays_mesh);
        take_owner!(entity_model_dragon_rays_depth_mesh);
        take_owner!(entity_model_end_portal_mesh);
        take_owner!(entity_model_end_gateway_mesh);
        take_owner!(hud_win_portal_mesh);
        take_owner!(entity_model_outline_mesh);
        take_owner!(entity_model_outline_cull_mesh);
        take_owner!(entity_model_entity_glint_mesh);
        take_owner!(entity_model_armor_entity_glint_mesh);
        take_owner!(entity_dynamic_player_skin_solid_mesh);
        take_owner!(entity_dynamic_player_skin_cutout_mesh);
        take_owner!(entity_dynamic_player_skin_cutout_cull_mesh);
        take_owner!(entity_dynamic_player_skin_solid_z_offset_forward_mesh);
        take_owner!(entity_dynamic_player_skin_cutout_z_offset_mesh);
        take_owner!(entity_dynamic_player_skin_translucent_mesh);
        take_owner!(entity_dynamic_player_skin_item_entity_translucent_mesh);
        take_owner!(entity_dynamic_player_skin_item_entity_translucent_cull_mesh);
        take_owner!(entity_dynamic_player_texture_solid_mesh);
        take_owner!(entity_dynamic_player_texture_cutout_mesh);
        take_owner!(entity_dynamic_player_texture_cutout_cull_mesh);
        take_owner!(entity_dynamic_player_texture_solid_z_offset_forward_mesh);
        take_owner!(entity_dynamic_player_texture_cutout_z_offset_mesh);
        take_owner!(entity_dynamic_player_texture_armor_cutout_mesh);
        take_owner!(entity_dynamic_player_texture_armor_trim_mesh);
        take_owner!(entity_dynamic_player_texture_armor_trim_decal_mesh);
        take_owner!(entity_dynamic_player_texture_translucent_mesh);
        take_owner!(entity_dynamic_player_texture_item_entity_translucent_mesh);
        take_owner!(entity_dynamic_player_texture_item_entity_translucent_cull_mesh);
        take_owner!(first_person_player_arm_mesh);
        take_owner!(first_person_dynamic_player_arm_mesh);
        take_owner!(entity_model_scroll_mesh);
        take_owner!(entity_model_scroll_additive_mesh);
        take_owner!(entity_model_guardian_beam_mesh);
        take_owner!(entity_model_end_crystal_beam_mesh);
        take_owner!(entity_model_end_gateway_beam_opaque_mesh);
        take_owner!(entity_model_end_gateway_beam_translucent_mesh);
        take_owner!(entity_model_bounds);

        // Sky/environment owners rebuilt from pack resources.
        take_owner!(end_sky_texture);
        take_owner!(sky_celestials);
        take_owner!(celestial_atlas);
        take_owner!(cloud_texture);
        take_owner!(clouds);
        take_owner!(weather_rain_texture);
        take_owner!(weather_snow_texture);
        take_owner!(world_border_texture);

        take_hud_resource_owners!(take_owner);
        // Wrapped chat lines are presentation state derived from both the
        // canonical message cache and the active font metrics. Keep them live
        // while staging, but retain an exact clone so a failed font reload
        // cannot leave new-generation wrapping paired with old glyph owners.
        clone_owner!(hud_chat_lines);

        self.resource_reload_checkpoint = Some(RendererResourceReloadCheckpoint {
            restores,
            previous_counters: self.counters.clone(),
        });
    }

    fn rollback_resource_reload(&mut self) {
        let checkpoint = self
            .resource_reload_checkpoint
            .take()
            .expect("resource reload rollback without a checkpoint");
        self.counters = checkpoint.previous_counters;
        for restore in checkpoint.restores.into_iter().rev() {
            restore(self);
        }
        // Cloud dimensions are written to a permanent uniform buffer. Restore
        // its contents after restoring the exact previous CPU texture owner.
        self.restore_cloud_uniform_after_resource_reload();
    }
}

// Keep the exhaustive HUD owner list in one macro so adding a reloadable HUD
// field requires one auditable entry rather than a hand-written backup type and
// a second restore assignment.
macro_rules! take_hud_resource_owners {
    ($take_owner:ident) => {
        $take_owner!(hud_vignette_texture);
        $take_owner!(hud_win_logo);
        $take_owner!(hud_win_easter_egg_logo);
        $take_owner!(hud_win_edition);
        $take_owner!(hud_win_credits_vignette);
        $take_owner!(hud_demo_popup_background);
        $take_owner!(hud_spyglass_scope_texture);
        $take_owner!(hud_crosshair);
        $take_owner!(hud_crosshair_attack_indicator_full);
        $take_owner!(hud_crosshair_attack_indicator_background);
        $take_owner!(hud_crosshair_attack_indicator_progress);
        $take_owner!(hud_hotbar);
        $take_owner!(hud_hotbar_selection);
        $take_owner!(hud_spectator_close);
        $take_owner!(hud_spectator_scroll_left);
        $take_owner!(hud_spectator_scroll_right);
        $take_owner!(hud_spectator_teleport_to_player);
        $take_owner!(hud_spectator_teleport_to_team);
        $take_owner!(hud_default_player_faces);
        $take_owner!(hud_dynamic_player_faces);
        $take_owner!(hud_player_list_ping_sprites);
        $take_owner!(hud_social_interactions_background);
        $take_owner!(hud_social_interactions_search);
        $take_owner!(hud_social_interactions_mute);
        $take_owner!(hud_social_interactions_mute_highlighted);
        $take_owner!(hud_social_interactions_unmute);
        $take_owner!(hud_social_interactions_unmute_highlighted);
        $take_owner!(hud_item_atlas);
        $take_owner!(hud_digit_atlas);
        $take_owner!(hud_digit_glyphs);
        $take_owner!(hud_font_atlas);
        $take_owner!(hud_font_glyphs);
        $take_owner!(hud_font_book);
        $take_owner!(hud_obfuscated_glyph_pool);
        $take_owner!(hud_chat_modified_icon);
        $take_owner!(hud_inworld_menu_background);
        $take_owner!(hud_inventory_background);
        $take_owner!(hud_creative_items_background);
        $take_owner!(hud_creative_item_search_background);
        $take_owner!(hud_creative_inventory_background);
        $take_owner!(hud_creative_scroller);
        $take_owner!(hud_creative_scroller_disabled);
        $take_owner!(hud_creative_top_unselected_tabs);
        $take_owner!(hud_creative_top_selected_tabs);
        $take_owner!(hud_creative_bottom_unselected_tabs);
        $take_owner!(hud_creative_bottom_selected_tabs);
        $take_owner!(hud_tooltip_background);
        $take_owner!(hud_tooltip_frame);
        $take_owner!(hud_tooltip_styles);
        $take_owner!(hud_generic_container_background);
        $take_owner!(hud_dispenser_background);
        $take_owner!(hud_crafting_table_background);
        $take_owner!(hud_cartography_table_background);
        $take_owner!(hud_cartography_table_error);
        $take_owner!(hud_cartography_table_scaled_map);
        $take_owner!(hud_cartography_table_duplicated_map);
        $take_owner!(hud_cartography_table_map);
        $take_owner!(hud_cartography_table_locked);
        $take_owner!(hud_loom_background);
        $take_owner!(hud_loom_banner_slot);
        $take_owner!(hud_loom_dye_slot);
        $take_owner!(hud_loom_pattern_slot);
        $take_owner!(hud_loom_scroller);
        $take_owner!(hud_loom_scroller_disabled);
        $take_owner!(hud_loom_pattern_selected);
        $take_owner!(hud_loom_pattern_highlighted);
        $take_owner!(hud_loom_pattern);
        $take_owner!(hud_loom_error);
        $take_owner!(hud_crafter_background);
        $take_owner!(hud_crafter_disabled_slot);
        $take_owner!(hud_crafter_powered_redstone);
        $take_owner!(hud_crafter_unpowered_redstone);
        $take_owner!(hud_widget_text_field);
        $take_owner!(hud_widget_text_field_highlighted);
        $take_owner!(hud_widget_preedit);
        $take_owner!(hud_widget_button);
        $take_owner!(hud_widget_button_disabled);
        $take_owner!(hud_widget_button_highlighted);
        $take_owner!(hud_dialog_warning_button);
        $take_owner!(hud_dialog_warning_button_disabled);
        $take_owner!(hud_dialog_warning_button_highlighted);
        $take_owner!(hud_widget_checkbox);
        $take_owner!(hud_widget_checkbox_highlighted);
        $take_owner!(hud_widget_checkbox_selected);
        $take_owner!(hud_widget_checkbox_selected_highlighted);
        $take_owner!(hud_widget_slider);
        $take_owner!(hud_widget_slider_highlighted);
        $take_owner!(hud_widget_slider_handle);
        $take_owner!(hud_widget_slider_handle_highlighted);
        $take_owner!(hud_stats_sprites);
        $take_owner!(hud_stats_scrollbar_sprites);
        $take_owner!(hud_system_toast_background);
        $take_owner!(hud_advancement_toast_background);
        $take_owner!(hud_recipe_toast_background);
        $take_owner!(hud_tutorial_toast_background);
        $take_owner!(hud_tutorial_toast_icons);
        $take_owner!(hud_debug_game_mode_switcher_background);
        $take_owner!(hud_debug_game_mode_switcher_slot);
        $take_owner!(hud_debug_game_mode_switcher_selection);
        $take_owner!(hud_anvil_background);
        $take_owner!(hud_anvil_text_field);
        $take_owner!(hud_anvil_text_field_disabled);
        $take_owner!(hud_anvil_error);
        $take_owner!(hud_enchanting_table_background);
        $take_owner!(hud_enchanting_table_lapis_slot);
        $take_owner!(hud_enchanting_table_enchantment_slot_disabled);
        $take_owner!(hud_enchanting_table_enchantment_slot_highlighted);
        $take_owner!(hud_enchanting_table_enchantment_slot);
        $take_owner!(hud_enchanting_table_level_1);
        $take_owner!(hud_enchanting_table_level_2);
        $take_owner!(hud_enchanting_table_level_3);
        $take_owner!(hud_enchanting_table_level_1_disabled);
        $take_owner!(hud_enchanting_table_level_2_disabled);
        $take_owner!(hud_enchanting_table_level_3_disabled);
        $take_owner!(hud_beacon_background);
        $take_owner!(hud_beacon_button_disabled);
        $take_owner!(hud_beacon_button_selected);
        $take_owner!(hud_beacon_button_highlighted);
        $take_owner!(hud_beacon_button);
        $take_owner!(hud_beacon_confirm);
        $take_owner!(hud_beacon_cancel);
        $take_owner!(hud_mob_effect_sprites);
        $take_owner!(hud_effect_background);
        $take_owner!(hud_effect_background_ambient);
        $take_owner!(hud_inventory_effect_background);
        $take_owner!(hud_inventory_effect_background_ambient);
        $take_owner!(hud_brewing_stand_background);
        $take_owner!(hud_brewing_stand_fuel_length);
        $take_owner!(hud_brewing_stand_brew_progress);
        $take_owner!(hud_brewing_stand_bubbles);
        $take_owner!(hud_furnace_background);
        $take_owner!(hud_furnace_lit_progress);
        $take_owner!(hud_furnace_burn_progress);
        $take_owner!(hud_advancements_window);
        $take_owner!(hud_advancement_tabs);
        $take_owner!(hud_advancement_backgrounds);
        $take_owner!(hud_advancement_widget_frames);
        $take_owner!(hud_advancement_hover_boxes);
        $take_owner!(hud_recipe_book_background);
        $take_owner!(hud_recipe_book_tab);
        $take_owner!(hud_recipe_book_tab_selected);
        $take_owner!(hud_recipe_book_button);
        $take_owner!(hud_recipe_book_button_highlighted);
        $take_owner!(hud_recipe_book_filter_enabled);
        $take_owner!(hud_recipe_book_filter_disabled);
        $take_owner!(hud_recipe_book_filter_enabled_highlighted);
        $take_owner!(hud_recipe_book_filter_disabled_highlighted);
        $take_owner!(hud_recipe_book_furnace_filter_enabled);
        $take_owner!(hud_recipe_book_furnace_filter_disabled);
        $take_owner!(hud_recipe_book_furnace_filter_enabled_highlighted);
        $take_owner!(hud_recipe_book_furnace_filter_disabled_highlighted);
        $take_owner!(hud_recipe_book_slot_craftable);
        $take_owner!(hud_recipe_book_slot_uncraftable);
        $take_owner!(hud_recipe_book_slot_many_craftable);
        $take_owner!(hud_recipe_book_slot_many_uncraftable);
        $take_owner!(hud_recipe_book_page_forward);
        $take_owner!(hud_recipe_book_page_forward_highlighted);
        $take_owner!(hud_recipe_book_page_backward);
        $take_owner!(hud_recipe_book_page_backward_highlighted);
        $take_owner!(hud_recipe_book_overlay_recipe);
        $take_owner!(hud_recipe_book_crafting_overlay);
        $take_owner!(hud_recipe_book_crafting_overlay_highlighted);
        $take_owner!(hud_recipe_book_crafting_overlay_disabled);
        $take_owner!(hud_recipe_book_crafting_overlay_disabled_highlighted);
        $take_owner!(hud_recipe_book_furnace_overlay);
        $take_owner!(hud_recipe_book_furnace_overlay_highlighted);
        $take_owner!(hud_recipe_book_furnace_overlay_disabled);
        $take_owner!(hud_recipe_book_furnace_overlay_disabled_highlighted);
        $take_owner!(hud_blast_furnace_background);
        $take_owner!(hud_blast_furnace_lit_progress);
        $take_owner!(hud_blast_furnace_burn_progress);
        $take_owner!(hud_smoker_background);
        $take_owner!(hud_smoker_lit_progress);
        $take_owner!(hud_smoker_burn_progress);
        $take_owner!(hud_smithing_background);
        $take_owner!(hud_smithing_error);
        $take_owner!(hud_smithing_slot_icons);
        $take_owner!(hud_grindstone_background);
        $take_owner!(hud_grindstone_error);
        $take_owner!(hud_hopper_background);
        $take_owner!(hud_horse_background);
        $take_owner!(hud_nautilus_background);
        $take_owner!(hud_mount_slot);
        $take_owner!(hud_mount_saddle_slot);
        $take_owner!(hud_mount_horse_armor_slot);
        $take_owner!(hud_mount_llama_armor_slot);
        $take_owner!(hud_mount_nautilus_armor_slot);
        $take_owner!(hud_mount_chest_slots);
        $take_owner!(hud_book_background);
        $take_owner!(hud_page_backward);
        $take_owner!(hud_page_backward_highlighted);
        $take_owner!(hud_page_forward);
        $take_owner!(hud_page_forward_highlighted);
        $take_owner!(hud_hanging_sign_backgrounds);
        $take_owner!(hud_shulker_box_background);
        $take_owner!(hud_stonecutter_background);
        $take_owner!(hud_stonecutter_scroller);
        $take_owner!(hud_stonecutter_scroller_disabled);
        $take_owner!(hud_stonecutter_recipe_selected);
        $take_owner!(hud_stonecutter_recipe_highlighted);
        $take_owner!(hud_stonecutter_recipe);
        $take_owner!(hud_villager_background);
        $take_owner!(hud_villager_out_of_stock);
        $take_owner!(hud_villager_experience_bar_background);
        $take_owner!(hud_villager_experience_bar_current);
        $take_owner!(hud_villager_experience_bar_result);
        $take_owner!(hud_villager_scroller);
        $take_owner!(hud_villager_scroller_disabled);
        $take_owner!(hud_villager_trade_arrow);
        $take_owner!(hud_villager_trade_arrow_out_of_stock);
        $take_owner!(hud_villager_discount_strikethrough);
        $take_owner!(hud_slot_highlight_back);
        $take_owner!(hud_slot_highlight_front);
        $take_owner!(hud_experience_background);
        $take_owner!(hud_experience_progress);
        $take_owner!(hud_jump_bar_background);
        $take_owner!(hud_jump_bar_progress);
        $take_owner!(hud_jump_bar_cooldown);
        $take_owner!(hud_locator_background);
        $take_owner!(hud_locator_arrow_up);
        $take_owner!(hud_locator_arrow_down);
        $take_owner!(hud_locator_marker_sprites);
        $take_owner!(hud_locator_styles);
        $take_owner!(hud_heart_sprites);
        $take_owner!(hud_food_empty);
        $take_owner!(hud_food_full);
        $take_owner!(hud_food_half);
        $take_owner!(hud_food_empty_hunger);
        $take_owner!(hud_food_full_hunger);
        $take_owner!(hud_food_half_hunger);
        $take_owner!(hud_armor_empty);
        $take_owner!(hud_armor_half);
        $take_owner!(hud_armor_full);
        $take_owner!(hud_air_bubble);
        $take_owner!(hud_air_bubble_bursting);
        $take_owner!(hud_air_bubble_empty);
        $take_owner!(hud_heart_vehicle_container);
        $take_owner!(hud_heart_vehicle_full);
        $take_owner!(hud_heart_vehicle_half);
        $take_owner!(hud_boss_bar_backgrounds);
        $take_owner!(hud_boss_bar_progress_sprites);
        $take_owner!(hud_boss_bar_notched_backgrounds);
        $take_owner!(hud_boss_bar_notched_progress_sprites);
    };
}

pub(crate) use take_hud_resource_owners;

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};

    use crate::{
        entity_model_texture_refs, ClearColor, EntityModelTextureImage, HudAsciiGlyph,
        HudChatMessage, HudComponentTextRun, HudCrosshairAttackIndicator, HudFontGlyphBook,
        HudFontGlyphMap, HudVignette, ParticleSpriteUv, ParticleUvRect, Renderer,
    };

    #[test]
    fn reload_checkpoint_captures_both_glint_texture_owners() {
        let source = include_str!("resource_reload.rs");
        let item = source
            .find("take_owner!(item_glint_texture)")
            .expect("item glint reload owner");
        let armor = source
            .find("take_owner!(armor_glint_texture)")
            .expect("armor glint reload owner");
        let next = source[armor + "take_owner!(armor_glint_texture)".len()..]
            .find("take_owner!(")
            .map(|index| armor + "take_owner!(armor_glint_texture)".len() + index)
            .expect("next reload owner");
        assert!(
            item < armor && armor < next,
            "item and armor glint owners are captured together"
        );
    }

    #[test]
    fn injected_failure_after_each_reload_stage_restores_exact_accepted_owners() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let old_rgba = [255, 0, 0, 255].repeat(16);
        renderer
            .upload_hud_crosshair(4, 4, &old_rgba)
            .expect("old crosshair");
        renderer
            .upload_hud_crosshair_attack_indicator_full(16, 16, &[0, 255, 0, 255].repeat(256))
            .expect("old full attack indicator");
        renderer.set_hud_crosshair_attack_indicator(HudCrosshairAttackIndicator::Full);
        for fail_after in 0..6 {
            let previous_counters = renderer.counters().clone();
            let error = renderer
                .transactional_resource_reload(|renderer| -> Result<()> {
                    renderer.upload_particle_atlas(
                        1,
                        1,
                        &[255, 255, 255, 255],
                        vec![ParticleSpriteUv {
                            id: "minecraft:test".to_owned(),
                            uv: ParticleUvRect {
                                min: [0.0, 0.0],
                                max: [1.0, 1.0],
                            },
                            has_translucent: false,
                        }],
                    )?;
                    renderer.upload_item_entity_atlas(1, 1, &[255, 255, 255, 255])?;
                    fail_at(0, fail_after)?;

                    renderer.upload_terrain_texture_atlas(2, 2, &[255; 16])?;
                    fail_at(1, fail_after)?;

                    renderer.upload_hud_crosshair(2, 2, &[0, 0, 255, 255].repeat(4))?;
                    renderer.upload_hud_crosshair_attack_indicator_full(
                        16,
                        16,
                        &[255, 0, 255, 255].repeat(256),
                    )?;
                    fail_at(2, fail_after)?;

                    let texture = entity_model_texture_refs()[0];
                    let rgba = vec![255; (texture.size[0] * texture.size[1] * 4) as usize];
                    renderer.upload_entity_model_textures(&[EntityModelTextureImage::new(
                        texture, rgba,
                    )])?;
                    fail_at(3, fail_after)?;

                    renderer.upload_end_sky_texture(1, 1, &[255, 255, 255, 255])?;
                    fail_at(4, fail_after)?;

                    renderer.invalidate_map_texture_residency();
                    renderer.invalidate_item_model_resources();
                    fail_at(5, fail_after)
                })
                .expect_err("injected stage failure");
            assert_eq!(
                error.to_string(),
                format!("injected failure after stage {fail_after}")
            );
            assert!(renderer.particle_atlas.is_none());
            assert!(renderer.item_entity_atlas.is_none());
            assert!(renderer.entity_model_texture_atlas.is_none());
            assert!(renderer.end_sky_texture.is_none());
            assert_eq!(renderer.counters(), previous_counters);

            let pixels = renderer
                .render_offscreen_frame()
                .expect("frame after rollback");
            let center = pixels.pixel(16, 16);
            assert!(
                center[0] > 128 && center[2] < 128,
                "rollback should retain the old red HUD owner, got {center:?}"
            );
            let attack_indicator = pixels.pixel(16, 25);
            assert!(
                attack_indicator[1] > 128 && attack_indicator[0] < 128 && attack_indicator[2] < 128,
                "rollback should retain the old green attack indicator, got {attack_indicator:?}"
            );
        }
    }

    #[test]
    fn successful_reload_commits_new_owner_and_discards_checkpoint() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer
            .upload_hud_crosshair(4, 4, &[255, 0, 0, 255].repeat(16))
            .expect("old crosshair");

        renderer
            .transactional_resource_reload(|renderer| {
                renderer.upload_hud_crosshair(4, 4, &[0, 0, 255, 255].repeat(16))
            })
            .expect("commit reload");

        assert!(renderer.resource_reload_checkpoint.is_none());
        let pixels = renderer
            .render_offscreen_frame()
            .expect("frame after commit");
        let center = pixels.pixel(16, 16);
        assert!(
            center[2] > 128 && center[0] < 128,
            "commit should expose the new blue HUD owner, got {center:?}"
        );
    }

    #[test]
    fn failed_vignette_reload_restores_the_previous_pixel_owner() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_clear_color(ClearColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_hud_vignette(Some(HudVignette {
            brightness: 0.5,
            border_warning_strength: 0.5,
            color: [0.25, 0.5, 0.75],
        }));
        renderer
            .upload_hud_vignette_texture(1, 1, &[255, 255, 255, 255], true, false)
            .expect("old vignette");
        let before = renderer
            .render_offscreen_frame()
            .expect("frame before reload")
            .pixel(2, 2);

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.upload_hud_vignette_texture(1, 1, &[0, 0, 0, 255], true, false)?;
                Err(anyhow!("injected vignette failure"))
            })
            .expect_err("injected reload failure");

        let after = renderer
            .render_offscreen_frame()
            .expect("frame after rollback")
            .pixel(2, 2);
        assert_eq!(after, before);
    }

    #[test]
    fn failed_spyglass_scope_reload_restores_the_previous_pixel_owner() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_hud_spyglass_scope(Some(crate::HudSpyglassScope { scale: 1.0 }));
        renderer
            .upload_hud_spyglass_scope(1, 1, &[255, 0, 0, 255])
            .expect("old scope");
        let before = renderer
            .render_offscreen_frame()
            .expect("frame before reload")
            .pixel(8, 8);

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.upload_hud_spyglass_scope(1, 1, &[0, 0, 255, 255])?;
                Err(anyhow!("injected scope failure"))
            })
            .expect_err("injected reload failure");

        let after = renderer
            .render_offscreen_frame()
            .expect("frame after rollback")
            .pixel(8, 8);
        assert_eq!(after, before);
        assert!(after[0] > 128 && after[2] < 128);
    }

    #[test]
    fn failed_player_list_ping_reload_restores_the_previous_owner() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer
            .upload_hud_player_list_ping_sprite(
                crate::HudPlayerListPingSprite::Unknown,
                10,
                8,
                &[255, 0, 0, 255].repeat(80),
            )
            .expect("old ping sprite");

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.upload_hud_player_list_ping_sprite(
                    crate::HudPlayerListPingSprite::Unknown,
                    2,
                    2,
                    &[0, 0, 255, 255].repeat(4),
                )?;
                Err(anyhow!("injected player-list ping failure"))
            })
            .expect_err("player-list ping reload should fail");

        let restored = renderer.hud_player_list_ping_sprites
            [crate::HudPlayerListPingSprite::Unknown as usize]
            .as_ref()
            .expect("restored ping sprite");
        assert_eq!((restored.width, restored.height), (10, 8));
    }

    #[test]
    fn failed_hud_font_reload_restores_exact_book_and_wrapped_chat_cache() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let mut old_glyphs = HudFontGlyphMap::new();
        old_glyphs.insert_first_wins(
            'a',
            HudAsciiGlyph {
                advance: 6,
                ..HudAsciiGlyph::default()
            },
        );
        let mut old_custom_glyphs = HudFontGlyphMap::new();
        old_custom_glyphs.insert_first_wins(
            'a',
            HudAsciiGlyph {
                advance: 9,
                ..HudAsciiGlyph::default()
            },
        );
        let old_fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([
                ("example:old".to_string(), old_custom_glyphs),
                ("minecraft:default".to_string(), old_glyphs),
            ]),
        );
        renderer
            .upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], old_fonts)
            .expect("old font");
        renderer.set_default_text_right_to_left(false);
        renderer.set_hud_chat_messages(vec![HudChatMessage {
            display_sequence: 1,
            added_tick: 0,
            runs: vec![HudComponentTextRun {
                visual: crate::HudStyledTextRun::plain("a".repeat(60)),
                font: Some("example:old".to_string()),
            }],
            interactions: Vec::new(),
            tag: None,
        }]);
        let previous_lines = renderer.hud_chat_lines.clone();
        let previous_fonts = renderer.hud_font_book.clone();

        let error = renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                let mut new_glyphs = HudFontGlyphMap::new();
                new_glyphs.insert_first_wins(
                    'a',
                    HudAsciiGlyph {
                        advance: 1,
                        ..HudAsciiGlyph::default()
                    },
                );
                let new_fonts = HudFontGlyphBook::from_fonts(
                    "minecraft:default",
                    std::collections::BTreeMap::from([(
                        "minecraft:default".to_string(),
                        new_glyphs,
                    )]),
                );
                renderer.upload_hud_font_atlas(1, 1, &[255, 255, 255, 255], new_fonts)?;
                renderer.set_default_text_right_to_left(true);
                assert_ne!(renderer.hud_chat_lines, previous_lines);
                assert_ne!(renderer.hud_font_book, previous_fonts);
                assert!(renderer.default_text_right_to_left());
                Err(anyhow!("injected font failure"))
            })
            .expect_err("font reload failure");

        assert_eq!(error.to_string(), "injected font failure");
        assert_eq!(renderer.hud_chat_lines, previous_lines);
        assert_eq!(renderer.hud_font_book, previous_fonts);
        assert!(!renderer.default_text_right_to_left());
    }

    #[test]
    fn failed_waypoint_locator_reload_restores_background_style_and_marker_sprite() {
        let Some(mut renderer) = Renderer::new_offscreen(320, 240) else {
            return;
        };
        let solid =
            |count: usize, rgba: [u8; 4]| -> Vec<u8> { (0..count).flat_map(|_| rgba).collect() };
        renderer
            .upload_hud_locator_background(182, 5, &solid(182 * 5, [255, 0, 0, 255]))
            .expect("old background");
        renderer
            .upload_hud_locator_marker_sprite(
                "old".to_string(),
                9,
                9,
                &solid(9 * 9, [0, 255, 0, 255]),
            )
            .expect("old marker");
        renderer.set_hud_locator_styles(std::collections::BTreeMap::from([(
            "test".to_string(),
            crate::HudLocatorStyle {
                near_distance: 1,
                far_distance: 2,
                sprite_ids: vec!["old".to_string()],
            },
        )]));
        renderer.set_hud_contextual_bar(crate::HudContextualBar::Locator(crate::HudLocatorBar {
            markers: vec![crate::HudLocatorMarker {
                angle_degrees: 0.0,
                pitch: crate::HudLocatorPitch::None,
                distance: 0.0,
                style_id: "test".to_string(),
                color_argb: 0xFFFF_FFFF,
            }],
        }));

        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.upload_hud_locator_background(
                    182,
                    5,
                    &solid(182 * 5, [0, 0, 255, 255]),
                )?;
                renderer.upload_hud_locator_marker_sprite(
                    "new".to_string(),
                    9,
                    9,
                    &solid(9 * 9, [255, 0, 255, 255]),
                )?;
                renderer.set_hud_locator_styles(std::collections::BTreeMap::from([(
                    "test".to_string(),
                    crate::HudLocatorStyle {
                        near_distance: 1,
                        far_distance: 2,
                        sprite_ids: vec!["new".to_string()],
                    },
                )]));
                Err(anyhow!("injected locator failure"))
            })
            .expect_err("locator reload should fail");

        assert!(renderer.hud_locator_marker_sprites.contains_key("old"));
        assert!(!renderer.hud_locator_marker_sprites.contains_key("new"));
        let pixels = renderer
            .render_offscreen_frame()
            .expect("frame after rollback");
        let background = pixels.pixel(73, 213);
        assert!(
            background[0] > 128 && background[1] < 128 && background[2] < 128,
            "rollback should restore the old red locator background, got {background:?}"
        );
        let marker = pixels.pixel(160, 214);
        assert!(
            marker[1] > 128 && marker[0] < 128 && marker[2] < 128,
            "rollback should restore the old green locator marker/style, got {marker:?}"
        );
    }

    #[test]
    fn every_pack_reloadable_hud_owner_is_checkpointed() {
        let hud_source = include_str!("hud.rs");
        let uploads = &hud_source[hud_source
            .find("    pub fn upload_hud_crosshair")
            .expect("HUD upload boundary")
            ..hud_source
                .find("    pub fn set_hud_code_of_conduct_screen")
                .expect("HUD presentation-state boundary")];
        let checkpoint_source = include_str!("resource_reload.rs");
        let mut owners = std::collections::BTreeSet::new();
        for suffix in uploads.split("self.").skip(1) {
            let owner = suffix
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .collect::<String>();
            if owner.starts_with("hud_") && owner != "hud_bind_group_layout" {
                owners.insert(owner);
            }
        }

        for owner in owners {
            assert!(
                checkpoint_source.contains(&format!("take_owner!({owner})"))
                    || checkpoint_source.contains(&format!("clone_owner!({owner})")),
                "reloadable HUD owner {owner} is missing from the transaction checkpoint"
            );
        }
        for owner in [
            "hud_default_player_faces",
            "hud_dynamic_player_faces",
            "hud_player_list_ping_sprites",
        ] {
            assert!(
                checkpoint_source.contains(&format!("take_owner!({owner})")),
                "derived HUD owner {owner} is missing from the transaction checkpoint"
            );
        }
    }

    #[test]
    fn core_pipelines_have_one_private_generation_owner_and_one_field_mapping() {
        let reload = include_str!("resource_reload.rs");
        let production = reload
            .split_once("\n#[cfg(test)]")
            .expect("resource reload production/test boundary")
            .0;
        assert_eq!(
            production.matches("struct CorePipelineGeneration").count(),
            1
        );
        assert!(!production.contains("pub struct CorePipelineGeneration"));
        assert!(!production.contains("pub(crate) struct CorePipelineGeneration"));
        assert_eq!(
            production
                .matches("CorePipelineGeneration::prepare(")
                .count(),
            1
        );
        assert_eq!(production.matches(".swap_with_renderer(").count(), 2);

        let mapping = production
            .split_once("    fn swap_with_renderer(&mut self, renderer: &mut Renderer) {")
            .expect("single core pipeline mapping")
            .1
            .split_once("\n    }\n}\n\nimpl Renderer")
            .expect("core pipeline mapping boundary")
            .0;
        assert_eq!(
            mapping.matches("std::mem::swap(").count(),
            42,
            "41 GPU handles plus the publication generation must share one mapping"
        );
        for glint_owner in [
            "entity_model_entity_glint_pipeline",
            "entity_model_armor_entity_glint_pipeline",
            "item_model_glint_pipeline",
            "resident_item_model_glint_pipeline",
            "hud_item_glint_pipeline",
        ] {
            assert!(
                mapping.contains(glint_owner),
                "selected glint owner {glint_owner} is missing from the shared generation mapping"
            );
        }
        assert_eq!(
            production
                .matches("adapt_selected_entity_shaders(resources)")
                .count(),
            1,
            "all current entity-shader consumers must share one selected-source adapter"
        );
        assert_eq!(
            production
                .matches("adapt_selected_glint_shaders(resources)")
                .count(),
            1,
            "all current glint consumers must share one selected-source adapter"
        );

        let core_reload = production
            .split_once("    pub fn reload_core_shaders(")
            .expect("public core shader reload entry")
            .1
            .split_once("\n    pub fn core_shader_generation(")
            .expect("core shader generation query boundary")
            .0;
        assert!(!core_reload.contains("std::mem::replace"));
        for legacy in [
            ["previous", "entity"].join("_"),
            ["previous", "item"].join("_"),
            ["previous", "particle"].join("_"),
            ["next", "weather"].join("_"),
        ] {
            assert!(
                !core_reload.contains(&legacy),
                "legacy mirror path {legacy}"
            );
        }

        let core_sources = [
            include_str!("gpu.rs"),
            include_str!("entity_models.rs"),
            include_str!("entity_models/gpu.rs"),
            include_str!("item_models.rs"),
            include_str!("particles.rs"),
            include_str!("particles/gpu.rs"),
        ]
        .join("\n");
        for family in ["Terrain", "Entity", "Item", "Particle"] {
            let legacy_type = format!("{family}CorePipeline{}", "Generation");
            assert!(
                !core_sources.contains(&legacy_type),
                "family container must not restore generation owner {legacy_type}"
            );
        }
        for family in ["terrain", "entity", "item", "particle"] {
            let legacy_factory = format!("create_{family}_core_pipeline_{}", "generation");
            assert!(
                !core_sources.contains(&legacy_factory),
                "family factory must not restore generation API {legacy_factory}"
            );
        }
    }

    #[test]
    fn every_renderer_pipeline_owner_has_one_shader_reload_classification() {
        use std::collections::BTreeSet;

        let renderer = include_str!("renderer.rs");
        let all = renderer
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("pub(super) ")?
                    .strip_suffix(": wgpu::RenderPipeline,")
            })
            .collect::<BTreeSet<_>>();
        let reload = include_str!("resource_reload.rs");
        let production = reload
            .split_once("\n#[cfg(test)]")
            .expect("resource reload production/test boundary")
            .0;
        let mapping = production
            .split_once("    fn swap_with_renderer(&mut self, renderer: &mut Renderer) {")
            .expect("single core pipeline mapping")
            .1
            .split_once("\n    }\n}\n\nimpl Renderer")
            .expect("core pipeline mapping boundary")
            .0;
        let selected = mapping
            .split("&mut renderer.")
            .skip(1)
            .filter_map(|suffix| {
                suffix
                    .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                    .next()
            })
            .filter(|owner| owner.ends_with("_pipeline"))
            .collect::<BTreeSet<_>>();

        // These current surfaces map to a Vanilla static RenderPipeline and therefore must be
        // migrated into CorePipelineGeneration. The value is the exact Java owner / core shader.
        #[rustfmt::skip]
        let pending = [
            ("block_destroy_pipeline", "CRUMBLING/core/rendertype_crumbling"),
            ("debug_text_pipeline", "TEXT/core/rendertype_text"),
            ("entity_model_dragon_rays_pipeline", "DRAGON_RAYS/core/rendertype_lightning"),
            ("entity_model_dragon_rays_depth_pipeline", "DRAGON_RAYS_DEPTH/core/position"),
            ("entity_model_leash_pipeline", "LEASH/core/rendertype_leash"),
            ("entity_model_end_portal_pipeline", "END_PORTAL/core/rendertype_end_portal"),
            ("entity_model_end_gateway_pipeline", "END_GATEWAY/core/rendertype_end_portal"),
            ("hud_win_portal_pipeline", "END_PORTAL/core/rendertype_end_portal"),
            ("entity_model_water_mask_pipeline", "WATER_MASK/core/rendertype_water_mask"),
            ("entity_model_outline_pipeline", "OUTLINE_NO_CULL/core/rendertype_outline"),
            ("entity_model_outline_cull_pipeline", "OUTLINE_CULL/core/rendertype_outline"),
            ("entity_model_beacon_beam_opaque_pipeline", "BEACON_BEAM_OPAQUE/core/rendertype_beacon_beam"),
            ("entity_model_beacon_beam_translucent_pipeline", "BEACON_BEAM_TRANSLUCENT/core/rendertype_beacon_beam"),
            ("lightning_pipeline", "LIGHTNING/core/rendertype_lightning"),
            ("world_border_pipeline", "WORLD_BORDER/core/rendertype_world_border"),
            ("entity_shadow_pipeline", "ENTITY_SHADOW/core/rendertype_entity_shadow"),
            ("display_text_background_pipeline", "TEXT_BACKGROUND/core/rendertype_text_background"),
            ("display_text_pipeline", "TEXT/core/rendertype_text"),
            ("display_text_see_through_pipeline", "TEXT_SEE_THROUGH/core/rendertype_text_see_through"),
            ("resident_item_model_solid_no_cutout_pipeline", "SOLID_BLOCK/core/block"),
            ("resident_item_model_end_portal_pipeline", "END_PORTAL/core/rendertype_end_portal"),
            ("resident_item_model_end_gateway_pipeline", "END_GATEWAY/core/rendertype_end_portal"),
            ("resident_item_model_outline_pipeline", "OUTLINE_NO_CULL/core/rendertype_outline"),
            ("selection_pipeline", "LINES/core/rendertype_lines"),
            ("translucent_debug_line_pipeline", "LINES_TRANSLUCENT/core/rendertype_lines"),
            ("chunk_border_always_on_top_pipeline", "LINES/core/rendertype_lines"),
            ("lightmap_pipeline", "LIGHTMAP/core/lightmap"),
            ("entity_outline_composite_pipeline", "ENTITY_OUTLINE_BLIT/core/blit_screen"),
            ("sky_pipeline", "SKY/core/sky"),
            ("star_pipeline", "STARS/core/stars"),
            ("end_sky_pipeline", "END_SKY/core/position_tex_color"),
            ("celestial_pipeline", "CELESTIAL/core/position_tex"),
            ("cloud_flat_pipeline", "FLAT_CLOUDS/core/clouds"),
            ("cloud_fancy_pipeline", "CLOUDS/core/clouds"),
            ("hud_pipeline", "GUI_TEXTURED/core/gui_textured"),
            ("vignette_pipeline", "VIGNETTE/core/gui_textured"),
        ];
        // These handles have no one-to-one Vanilla static core owner: eight are exact/default
        // post-graph or renderer-internal copies whose selected path is PostEffectGraphRuntime;
        // two are BBB fallback presentation lanes absent from Vanilla's pipeline registry.
        #[rustfmt::skip]
        let fixed = [
            ("screen_blur_horizontal_pipeline", "minecraft:blur post fallback"),
            ("screen_blur_vertical_pipeline", "minecraft:blur post fallback"),
            ("transparency_combine_pipeline", "minecraft:transparency optimized default graph"),
            ("transparency_blit_pipeline", "renderer-internal final-target copy"),
            ("entity_model_pipeline", "BBB colored entity fallback"),
            ("item_entity_pipeline", "BBB billboard item fallback"),
            ("entity_outline_sobel_pipeline", "minecraft:entity_outline post fallback"),
            ("entity_outline_blur_horizontal_pipeline", "minecraft:entity_outline post fallback"),
            ("entity_outline_blur_vertical_pipeline", "minecraft:entity_outline post fallback"),
            ("entity_outline_blit_pipeline", "minecraft:entity_outline post fallback copy"),
        ];

        assert_eq!(all.len(), 87, "renderer pipeline inventory changed");
        assert_eq!(selected.len(), 41, "selected core generation changed");
        assert_eq!(
            pending.len(),
            36,
            "pending static-core classification changed"
        );
        assert_eq!(fixed.len(), 10, "fixed/fallback classification changed");
        let pending = pending
            .into_iter()
            .map(|(owner, _)| owner)
            .collect::<BTreeSet<_>>();
        let fixed = fixed
            .into_iter()
            .map(|(owner, _)| owner)
            .collect::<BTreeSet<_>>();
        assert!(selected.is_disjoint(&pending));
        assert!(selected.is_disjoint(&fixed));
        assert!(pending.is_disjoint(&fixed));
        let classified = selected
            .union(&pending)
            .copied()
            .collect::<BTreeSet<_>>()
            .union(&fixed)
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            classified, all,
            "every renderer pipeline needs one reload classification"
        );
    }

    #[test]
    fn renderer_production_dependency_points_at_pure_render_types_not_pack_loaders() {
        let manifest = include_str!("../Cargo.toml");
        let dependencies = manifest
            .split_once("[dependencies]")
            .expect("renderer dependency section")
            .1
            .split_once("[dev-dependencies]")
            .expect("renderer dev-dependency section")
            .0;
        assert!(dependencies.contains("bbb-render-types"));
        assert!(
            !dependencies.contains("bbb-pack"),
            "renderer production code must not regain a pack loader dependency"
        );
    }

    fn fail_at(stage: usize, fail_after: usize) -> Result<()> {
        if stage == fail_after {
            return Err(anyhow!("injected failure after stage {stage}"));
        }
        Ok(())
    }
}
