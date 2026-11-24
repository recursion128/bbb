//! Core-shader pipelines. `with_gpu` and the core-shader reload build them through one
//! constructor; a reload swaps the whole value.

use bbb_render_types::CoreShaderSourceProjection;

use super::*;
use crate::core_shaders as shaders;
use crate::sky::SkyPipelines;

/// The pack-selected core shader stages; `Default` is the embedded Vanilla set.
#[derive(Default, PartialEq)]
pub(crate) struct SelectedCoreShaders {
    pub(super) terrain: Option<shaders::AdaptedTerrainShaders>,
    entity: Option<shaders::AdaptedEntityShaders>,
    item: Option<shaders::AdaptedItemShaders>,
    particle: Option<shaders::AdaptedParticleShaders>,
    position_color: Option<shaders::AdaptedPositionColorShaders>,
    glint: Option<shaders::AdaptedGlintShaders>,
    end_portal: Option<shaders::AdaptedEndPortalShaders>,
    outline: Option<shaders::AdaptedOutlineShaders>,
    beacon_beam: Option<shaders::AdaptedBeaconBeamShaders>,
    lightning: Option<shaders::AdaptedLightningShaders>,
    entity_special: shaders::AdaptedEntitySpecialShaders,
    lines: Option<shaders::AdaptedLineShaders>,
    text: Option<shaders::AdaptedTextShaders>,
    text_background: Option<shaders::AdaptedTextBackgroundShaders>,
    pub(crate) clouds: Option<shaders::AdaptedCloudShaders>,
    lightmap: Option<shaders::AdaptedLightmapShaders>,
    sky: shaders::AdaptedSkyShaders,
    position_tex_color: Option<shaders::AdaptedPositionTexColorShaders>,
    gui: Option<shaders::AdaptedShaderPair>,
    static_core: shaders::AdaptedStaticCoreShaders,
}

impl SelectedCoreShaders {
    pub(crate) fn uses_embedded_entity(&self) -> bool {
        self.entity.is_none()
    }

    pub(crate) fn adapt(
        resources: &CoreShaderSourceProjection,
        texture_filtering: TerrainTextureFiltering,
        allow_embedded_fallback: bool,
    ) -> Result<Self> {
        if allow_embedded_fallback && resources.source_count() == 0 {
            return Ok(Self::default());
        }
        if !allow_embedded_fallback && !shaders::has_terrain_shader_sources(resources) {
            anyhow::bail!("prepared core shaders are missing minecraft:core/terrain");
        }
        Ok(Self {
            terrain: shaders::adapt_selected_terrain_shaders(
                resources,
                texture_filtering == TerrainTextureFiltering::Rgss,
            )?,
            entity: shaders::adapt_selected_entity_shaders(resources)?,
            item: shaders::adapt_selected_item_shaders(resources)?,
            particle: shaders::adapt_selected_particle_shaders(resources)?,
            position_color: shaders::adapt_selected_position_color_shaders(resources)?,
            glint: shaders::adapt_selected_glint_shaders(resources)?,
            end_portal: shaders::adapt_selected_end_portal_shaders(resources)?,
            outline: shaders::adapt_selected_outline_shaders(resources)?,
            beacon_beam: shaders::adapt_selected_beacon_beam_shaders(resources)?,
            lightning: shaders::adapt_selected_lightning_shaders(resources)?,
            entity_special: shaders::adapt_selected_entity_special_shaders(resources)?,
            lines: shaders::adapt_selected_line_shaders(resources)?,
            text: shaders::adapt_selected_text_shaders(resources)?,
            text_background: shaders::adapt_selected_text_background_shaders(resources)?,
            clouds: shaders::adapt_selected_cloud_shaders(resources)?,
            lightmap: shaders::adapt_selected_lightmap_shaders(resources)?,
            sky: shaders::adapt_selected_sky_shaders(resources)?,
            position_tex_color: shaders::adapt_selected_position_tex_color_shaders(resources)?,
            gui: shaders::adapt_selected_gui_shaders(resources)?,
            static_core: shaders::adapt_selected_static_core_shaders(resources)?,
        })
    }
}

/// The renderer layouts and target format every core pipeline is built against.
pub(crate) struct CorePipelineLayouts<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) terrain: &'a wgpu::BindGroupLayout,
    pub(crate) lightmap: &'a wgpu::BindGroupLayout,
    pub(crate) lightmap_sample: &'a wgpu::BindGroupLayout,
    pub(crate) hud: &'a wgpu::BindGroupLayout,
    pub(crate) gui_extent: [f32; 2],
    pub(crate) cloud: &'a wgpu::BindGroupLayout,
    pub(crate) vignette: &'a wgpu::BindGroupLayout,
    pub(crate) entity_outline: &'a wgpu::BindGroupLayout,
    pub(crate) sky: &'a SkyGpu,
    pub(crate) terrain_texture_filtering: TerrainTextureFiltering,
    pub(crate) dynamic_lights_mode: DynamicLightsMode,
}

pub(crate) struct CorePipelines {
    /// The selection these pipelines were built from; an equal selection reuses them.
    pub(crate) shaders: SelectedCoreShaders,
    pub(crate) terrain: TerrainCorePipelines,
    pub(crate) lod_terrain: wgpu::RenderPipeline,
    pub(crate) sky: SkyPipelines,
    pub(crate) lightmap: wgpu::RenderPipeline,
    pub(crate) cloud_flat: wgpu::RenderPipeline,
    pub(crate) cloud_fancy: wgpu::RenderPipeline,
    pub(crate) hud: wgpu::RenderPipeline,
    pub(crate) hud_gui: crate::hud::HudGuiPipelines,
    pub(crate) hud_nausea_overlay: wgpu::RenderPipeline,
    pub(crate) hud_crosshair: wgpu::RenderPipeline,
    pub(crate) hud_item_glint: wgpu::RenderPipeline,
    pub(crate) hud_win_portal: wgpu::RenderPipeline,
    pub(crate) vignette: wgpu::RenderPipeline,
    pub(crate) screen_effect: wgpu::RenderPipeline,
    pub(crate) entity_outline_composite: wgpu::RenderPipeline,
    pub(crate) block_destroy: wgpu::RenderPipeline,
    pub(crate) debug_filled_box: wgpu::RenderPipeline,
    pub(crate) debug_text: wgpu::RenderPipeline,
    pub(crate) entity_model_textured: wgpu::RenderPipeline,
    pub(crate) entity_model_textured_cull: wgpu::RenderPipeline,
    pub(crate) entity_model_solid: wgpu::RenderPipeline,
    pub(crate) entity_model_dissolve: wgpu::RenderPipeline,
    pub(crate) entity_model_solid_z_offset_forward: wgpu::RenderPipeline,
    pub(crate) entity_model_cutout_z_offset: wgpu::RenderPipeline,
    pub(crate) entity_model_armor_cutout: wgpu::RenderPipeline,
    pub(crate) entity_model_armor_decal: wgpu::RenderPipeline,
    pub(crate) entity_model_translucent: wgpu::RenderPipeline,
    pub(crate) entity_model_banner_pattern: wgpu::RenderPipeline,
    pub(crate) entity_model_translucent_cull: wgpu::RenderPipeline,
    pub(crate) entity_model_armor_translucent: wgpu::RenderPipeline,
    pub(crate) entity_model_translucent_emissive: wgpu::RenderPipeline,
    pub(crate) entity_model_eyes: wgpu::RenderPipeline,
    pub(crate) entity_model_dragon_rays: wgpu::RenderPipeline,
    pub(crate) entity_model_dragon_rays_depth: wgpu::RenderPipeline,
    pub(crate) entity_model_leash: wgpu::RenderPipeline,
    pub(crate) entity_model_end_portal: wgpu::RenderPipeline,
    pub(crate) entity_model_end_gateway: wgpu::RenderPipeline,
    pub(crate) entity_model_water_mask: wgpu::RenderPipeline,
    pub(crate) entity_model_outline: wgpu::RenderPipeline,
    pub(crate) entity_model_outline_cull: wgpu::RenderPipeline,
    pub(crate) entity_model_scroll: wgpu::RenderPipeline,
    pub(crate) entity_model_scroll_additive: wgpu::RenderPipeline,
    pub(crate) entity_model_guardian_beam: wgpu::RenderPipeline,
    pub(crate) entity_model_end_crystal_beam: wgpu::RenderPipeline,
    pub(crate) entity_model_beacon_beam_opaque: wgpu::RenderPipeline,
    pub(crate) entity_model_beacon_beam_translucent: wgpu::RenderPipeline,
    pub(crate) entity_model_entity_glint: wgpu::RenderPipeline,
    pub(crate) entity_model_armor_entity_glint: wgpu::RenderPipeline,
    pub(crate) opaque_particle: wgpu::RenderPipeline,
    pub(crate) translucent_particle: wgpu::RenderPipeline,
    pub(crate) weather_no_depth_write: wgpu::RenderPipeline,
    pub(crate) weather_depth_write: wgpu::RenderPipeline,
    pub(crate) lightning: wgpu::RenderPipeline,
    pub(crate) world_border: wgpu::RenderPipeline,
    pub(crate) item_entity: wgpu::RenderPipeline,
    pub(crate) item_model: wgpu::RenderPipeline,
    pub(crate) entity_shadow: wgpu::RenderPipeline,
    pub(crate) display_text_background: wgpu::RenderPipeline,
    pub(crate) display_text: wgpu::RenderPipeline,
    pub(crate) display_text_see_through: wgpu::RenderPipeline,
    pub(crate) resident_item_model: wgpu::RenderPipeline,
    pub(crate) resident_item_model_solid_no_cutout: wgpu::RenderPipeline,
    pub(crate) resident_item_model_z_offset_forward: wgpu::RenderPipeline,
    pub(crate) resident_item_model_cutout_no_cull: wgpu::RenderPipeline,
    pub(crate) resident_item_model_cutout_z_offset: wgpu::RenderPipeline,
    pub(crate) resident_item_model_pattern: wgpu::RenderPipeline,
    pub(crate) resident_item_model_translucent: wgpu::RenderPipeline,
    pub(crate) resident_item_model_translucent_no_cull: wgpu::RenderPipeline,
    pub(crate) resident_item_model_glint: wgpu::RenderPipeline,
    pub(crate) resident_item_model_end_portal: wgpu::RenderPipeline,
    pub(crate) resident_item_model_end_gateway: wgpu::RenderPipeline,
    pub(crate) resident_item_model_outline: wgpu::RenderPipeline,
    pub(crate) selection: wgpu::RenderPipeline,
    pub(crate) translucent_debug_line: wgpu::RenderPipeline,
    pub(crate) chunk_border_always_on_top: wgpu::RenderPipeline,
}

impl CorePipelines {
    pub(crate) fn new(l: &CorePipelineLayouts<'_>, s: SelectedCoreShaders) -> Result<Self> {
        let (device, format, terrain, lightmap) =
            (l.device, l.format, l.terrain, l.lightmap_sample);
        let crate::entity_models::EntityCorePipelines {
            textured: entity_model_textured,
            textured_cull: entity_model_textured_cull,
            solid: entity_model_solid,
            dissolve: entity_model_dissolve,
            solid_z_offset_forward: entity_model_solid_z_offset_forward,
            cutout_z_offset: entity_model_cutout_z_offset,
            armor_cutout: entity_model_armor_cutout,
            armor_decal: entity_model_armor_decal,
            armor_translucent: entity_model_armor_translucent,
            translucent: entity_model_translucent,
            banner_pattern: entity_model_banner_pattern,
            translucent_cull: entity_model_translucent_cull,
            translucent_emissive: entity_model_translucent_emissive,
            eyes: entity_model_eyes,
        } = crate::entity_models::create_entity_core_pipelines(
            device,
            format,
            terrain,
            lightmap,
            s.entity.as_ref(),
        )?;
        let crate::item_models::ItemCorePipelines {
            dynamic_cutout: item_model,
            resident_cutout: resident_item_model,
            resident_translucent: resident_item_model_translucent,
        } = crate::item_models::create_item_core_pipelines(
            device,
            format,
            terrain,
            lightmap,
            s.item.as_ref(),
        )?;
        let crate::particles::ParticleCorePipelines {
            opaque: opaque_particle,
            translucent: translucent_particle,
        } = crate::particles::create_particle_core_pipelines(
            device,
            format,
            terrain,
            lightmap,
            s.particle.as_ref(),
        )?;
        let entity = s.entity.as_ref();
        let glint = s.glint.as_ref();
        let end_portal = s.end_portal.as_ref();
        let outline = s.outline.as_ref();
        let beacon_beam = s.beacon_beam.as_ref();
        let lightning = s.lightning.as_ref();
        let entity_special = Some(&s.entity_special);
        let lines = s.lines.as_ref();
        let text = s.text.as_ref();
        let position_tex_color = s.position_tex_color.as_ref();
        let static_core = &s.static_core;
        Ok(Self {
            terrain: crate::gpu::create_terrain_core_pipelines(
                device,
                format,
                terrain,
                lightmap,
                s.terrain.as_ref(),
                l.terrain_texture_filtering,
                l.dynamic_lights_mode,
            )?,
            lod_terrain: crate::scene::lod_terrain::create_lod_terrain_pipeline(
                device, format, terrain, lightmap,
            ),
            sky: SkyPipelines {
                disc: crate::sky::create_sky_pipeline(
                    device,
                    format,
                    terrain,
                    &l.sky.dynamic_layout,
                    s.sky.sky.as_ref(),
                )?,
                sunrise_sunset: crate::sky::create_sunrise_sunset_core_pipeline(
                    device,
                    format,
                    terrain,
                    s.position_color.as_ref(),
                )?,
                star: crate::sky::create_star_pipeline(
                    device,
                    format,
                    terrain,
                    &l.sky.dynamic_layout,
                    s.sky.stars.as_ref(),
                )?,
                end_sky: crate::sky::create_end_sky_pipeline(
                    device,
                    format,
                    terrain,
                    &l.sky.end_sky_texture_layout,
                    &l.sky.dynamic_layout,
                    position_tex_color.map(|pair| &pair.end_sky),
                )?,
                celestial: crate::sky::create_celestial_pipeline(
                    device,
                    format,
                    terrain,
                    &l.sky.celestial_layout,
                    &l.sky.dynamic_layout,
                    s.sky.celestial.as_ref(),
                )?,
            },
            lightmap: crate::lightmap::create_lightmap_pipeline(
                device,
                l.lightmap,
                s.lightmap.as_ref(),
            )?,
            cloud_flat: crate::clouds::create_cloud_pipeline(
                device,
                format,
                terrain,
                l.cloud,
                crate::CloudShape::Flat,
                s.clouds.as_ref(),
            )?,
            cloud_fancy: crate::clouds::create_cloud_pipeline(
                device,
                format,
                terrain,
                l.cloud,
                crate::CloudShape::Fancy,
                s.clouds.as_ref(),
            )?,
            hud: crate::hud::create_hud_pipeline(
                device,
                format,
                l.hud,
                position_tex_color,
                wgpu::BlendState::ALPHA_BLENDING,
            )?,
            hud_crosshair: crate::hud::create_hud_pipeline(
                device,
                format,
                l.hud,
                position_tex_color,
                crate::hud::HUD_CROSSHAIR_BLEND,
            )?,
            hud_gui: crate::hud::create_hud_gui_pipelines(
                device,
                format,
                l.gui_extent,
                s.gui.as_ref(),
            )?,
            hud_nausea_overlay: crate::hud::create_hud_nausea_overlay_pipeline(
                device,
                format,
                l.hud,
                position_tex_color,
            )?,
            hud_item_glint: crate::hud::create_hud_item_glint_pipeline(
                device, format, terrain, l.hud, glint,
            )?,
            hud_win_portal: crate::entity_models::create_win_screen_end_portal_pipeline(
                device, format, terrain, end_portal,
            )?,
            vignette: crate::vignette::create_vignette_pipeline(
                device,
                format,
                l.vignette,
                position_tex_color,
            )?,
            screen_effect: crate::screen_effects::create_screen_effect_pipeline(
                device,
                format,
                l.hud,
                position_tex_color,
            )?,
            entity_outline_composite: crate::outline::create_entity_outline_composite_pipeline(
                device,
                format,
                l.entity_outline,
                static_core.entity_outline_blit.as_ref(),
            )?,
            block_destroy: crate::block_destroy::create_block_destroy_pipeline_with_shaders(
                device,
                format,
                terrain,
                static_core.crumbling.as_ref(),
            )?,
            debug_filled_box: crate::debug_filled_boxes::create_debug_filled_box_core_pipeline(
                device,
                format,
                terrain,
                s.position_color.as_ref(),
            )?,
            debug_text: crate::debug_text_billboards::create_debug_text_pipeline(
                device, format, terrain, lightmap, text,
            )?,
            entity_model_textured,
            entity_model_textured_cull,
            entity_model_solid,
            entity_model_dissolve,
            entity_model_solid_z_offset_forward,
            entity_model_cutout_z_offset,
            entity_model_armor_cutout,
            entity_model_armor_decal,
            entity_model_translucent,
            entity_model_banner_pattern,
            entity_model_translucent_cull,
            entity_model_armor_translucent,
            entity_model_translucent_emissive,
            entity_model_eyes,
            entity_model_dragon_rays: create_entity_model_dragon_rays_pipeline(
                device, format, terrain, lightning,
            )?,
            entity_model_dragon_rays_depth: create_entity_model_dragon_rays_depth_pipeline(
                device,
                format,
                terrain,
                entity_special,
            )?,
            entity_model_leash: create_entity_model_leash_pipeline(
                device,
                format,
                terrain,
                lightmap,
                entity_special,
            )?,
            entity_model_end_portal: create_entity_model_end_portal_pipeline(
                device, format, terrain, end_portal,
            )?,
            entity_model_end_gateway: create_entity_model_end_gateway_pipeline(
                device, format, terrain, end_portal,
            )?,
            entity_model_water_mask: create_entity_model_water_mask_pipeline(
                device,
                format,
                terrain,
                entity_special,
            )?,
            entity_model_outline: create_entity_model_outline_pipeline(
                device, format, terrain, outline,
            )?,
            entity_model_outline_cull: create_entity_model_outline_cull_pipeline(
                device, format, terrain, outline,
            )?,
            entity_model_scroll: create_entity_model_scroll_pipeline(
                device, format, terrain, lightmap, entity,
            )?,
            entity_model_scroll_additive: create_entity_model_scroll_additive_pipeline(
                device, format, terrain, entity,
            )?,
            entity_model_guardian_beam: create_entity_model_guardian_beam_pipeline(
                device, format, terrain, lightmap, entity,
            )?,
            entity_model_end_crystal_beam: create_entity_model_end_crystal_beam_pipeline(
                device, format, terrain, lightmap, entity,
            )?,
            entity_model_beacon_beam_opaque: create_entity_model_beacon_beam_opaque_pipeline(
                device,
                format,
                terrain,
                beacon_beam,
            )?,
            entity_model_beacon_beam_translucent:
                create_entity_model_beacon_beam_translucent_pipeline(
                    device,
                    format,
                    terrain,
                    beacon_beam,
                )?,
            entity_model_entity_glint: create_entity_model_entity_glint_pipeline(
                device, format, terrain, glint,
            )?,
            entity_model_armor_entity_glint: create_entity_model_armor_entity_glint_pipeline(
                device, format, terrain, glint,
            )?,
            opaque_particle,
            translucent_particle,
            weather_no_depth_write: crate::weather::create_weather_core_pipeline(
                device,
                format,
                terrain,
                lightmap,
                s.particle.as_ref(),
                crate::weather::WEATHER_PIPELINE_NO_DEPTH_WRITE,
            )?,
            weather_depth_write: crate::weather::create_weather_core_pipeline(
                device,
                format,
                terrain,
                lightmap,
                s.particle.as_ref(),
                crate::weather::WEATHER_PIPELINE_DEPTH_WRITE,
            )?,
            lightning: crate::weather::create_lightning_pipeline(
                device, format, terrain, lightning,
            )?,
            world_border: crate::world_border::create_world_border_pipeline_with_shaders(
                device,
                format,
                terrain,
                static_core.world_border.as_ref(),
            )?,
            item_entity: create_item_entity_pipeline(device, format, terrain, lightmap),
            item_model,
            entity_shadow: create_entity_shadow_pipeline(
                device,
                format,
                terrain,
                lightmap,
                entity_special,
            )?,
            display_text_background: create_display_text_background_pipeline(
                device,
                format,
                terrain,
                lightmap,
                s.text_background.as_ref(),
            )?,
            display_text: create_display_text_pipeline(device, format, terrain, lightmap, text)?,
            display_text_see_through: create_display_text_see_through_pipeline(
                device, format, terrain, lightmap, text,
            )?,
            resident_item_model,
            resident_item_model_solid_no_cutout:
                create_resident_item_model_solid_no_cutout_pipeline(
                    device,
                    format,
                    terrain,
                    lightmap,
                    static_core.resident_block.as_ref(),
                )?,
            resident_item_model_z_offset_forward:
                create_resident_item_model_z_offset_forward_pipeline(
                    device, format, terrain, lightmap, entity,
                )?,
            resident_item_model_cutout_no_cull: create_resident_item_model_cutout_no_cull_pipeline(
                device, format, terrain, lightmap, entity,
            )?,
            resident_item_model_cutout_z_offset:
                create_resident_item_model_cutout_z_offset_pipeline(
                    device, format, terrain, lightmap, entity,
                )?,
            resident_item_model_pattern: create_resident_item_model_pattern_pipeline(
                device, format, terrain, lightmap, entity,
            )?,
            resident_item_model_translucent,
            resident_item_model_translucent_no_cull:
                create_resident_item_model_translucent_no_cull_pipeline(
                    device, format, terrain, lightmap, entity,
                )?,
            resident_item_model_glint: create_resident_item_model_glint_pipeline(
                device, format, terrain, glint,
            )?,
            resident_item_model_end_portal: create_resident_item_model_end_portal_pipeline(
                device, format, terrain, end_portal,
            )?,
            resident_item_model_end_gateway: create_resident_item_model_end_gateway_pipeline(
                device, format, terrain, end_portal,
            )?,
            resident_item_model_outline: create_resident_item_model_outline_pipeline(
                device, format, terrain, outline,
            )?,
            selection: create_selection_pipeline(device, format, terrain, lines)?,
            translucent_debug_line: create_translucent_debug_line_pipeline(
                device, format, terrain, lines,
            )?,
            chunk_border_always_on_top: create_chunk_border_always_on_top_pipeline(
                device, format, terrain, lines,
            )?,
            shaders: s,
        })
    }
}

impl Renderer {
    // Selected shaders can read vertex IDs and flat attributes from the provoking vertex.
    pub fn uses_embedded_terrain_shader(&self) -> bool {
        self.pipelines.shaders.terrain.is_none()
    }

    pub(crate) fn uses_instanced_particles(&self) -> bool {
        self.pipelines.shaders.particle.is_none()
    }

    pub(crate) fn core_pipeline_layouts(&self) -> CorePipelineLayouts<'_> {
        CorePipelineLayouts {
            device: &self.device,
            format: self.config.format,
            terrain: &self.terrain_bind_group_layout,
            lightmap: &self.lightmap_bind_group_layout,
            lightmap_sample: &self.lightmap_sample_bind_group_layout,
            hud: &self.hud_bind_group_layout,
            gui_extent: self.gui_projection_extent(),
            cloud: &self.cloud_bind_group_layout,
            vignette: &self.vignette_bind_group_layout,
            entity_outline: &self.entity_outline_bind_group_layout,
            sky: &self.sky,
            terrain_texture_filtering: self.terrain_texture_filtering,
            dynamic_lights_mode: self.dynamic_lights_mode,
        }
    }
}
