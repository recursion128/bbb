use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

use anyhow::{anyhow, Result};
use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

use crate::frame_buffers::FrameDataBuffer;
use crate::{
    block_destroy::{
        create_block_destroy_overlays_gpu, create_block_destroy_pipeline, BlockDestroyOverlay,
        BlockDestroyOverlaysGpu,
    },
    camera::{
        sanitize_lightmap_block_factor, sanitize_lightmap_brightness_factor, CameraPose,
        CameraUniform, ClearColor, FogEnvironment, LightmapEnvironment, TerrainBounds,
        VANILLA_DEFAULT_GLINT_SPEED,
    },
    clouds::{
        cloud_mesh_key, create_cloud_bind_group, create_cloud_bind_group_layout, create_cloud_gpu,
        create_cloud_pipeline, create_cloud_target, create_cloud_texture_data,
        create_cloud_uniform_buffer, write_cloud_uniform, CloudEnvironment, CloudFrame, CloudGpu,
        CloudShape, CloudTarget, CloudTextureData, CloudTextureImage,
    },
    counters::RendererCounters,
    entity_models::{
        create_entity_model_armor_cutout_pipeline, create_entity_model_armor_entity_glint_pipeline,
        create_entity_model_armor_translucent_pipeline,
        create_entity_model_cutout_z_offset_pipeline, create_entity_model_dissolve_pipeline,
        create_entity_model_dragon_rays_depth_pipeline, create_entity_model_dragon_rays_pipeline,
        create_entity_model_entity_glint_pipeline, create_entity_model_eyes_pipeline,
        create_entity_model_outline_cull_pipeline, create_entity_model_outline_pipeline,
        create_entity_model_pipeline, create_entity_model_scroll_additive_pipeline,
        create_entity_model_scroll_pipeline, create_entity_model_textured_cull_pipeline,
        create_entity_model_textured_pipeline, create_entity_model_translucent_cull_pipeline,
        create_entity_model_translucent_emissive_pipeline,
        create_entity_model_translucent_pipeline, create_entity_model_water_mask_pipeline,
        EntityDynamicPlayerSkinAtlasGpu, EntityDynamicPlayerTextureAtlasGpu, EntityModelMeshGpu,
        EntityModelScrollMeshGpu, EntityModelTextureAtlasGpu, EntityModelTexturedDrawRange,
        EntityModelTexturedMeshGpu, EntityModelTranslucentDrawRange, FirstPersonPlayerArm,
        HudEntityPreviewPipTarget,
    },
    gpu::{
        create_camera_buffer, create_depth_target, create_terrain_atlas_gpu,
        create_terrain_atlas_mips_gpu, create_terrain_bind_group, create_terrain_bind_group_layout,
        create_terrain_pipeline, create_terrain_translucent_pipeline, write_terrain_atlas_gpu,
        write_terrain_atlas_mips_gpu, DepthTarget, TerrainAtlasGpu,
    },
    hud::{
        create_hud_bind_group_layout, create_hud_item_glint_pipeline, create_hud_pipeline,
        create_hud_sprite_gpu, HudAdvancementBackgroundTexture, HudAdvancementHoverBoxSprite,
        HudAdvancementTabSprite, HudAdvancementWidgetFrameSprite, HudDigitGlyph, HudFontGlyphMap,
        HudInventoryScreen, HudItemIcon, HudNineSliceSprite, HudObfuscatedGlyphPool, HudSpriteGpu,
        HUD_HOTBAR_SLOTS,
    },
    item_entities::{create_item_entity_pipeline, ItemEntityAtlasGpu, ItemEntityBillboard},
    item_models::{
        create_item_model_glint_pipeline, create_item_model_pipeline,
        create_item_model_translucent_pipeline, create_item_model_z_offset_forward_pipeline,
        FirstPersonMapBackgroundAtlasGpu, FirstPersonMapBackgroundSurface, ItemFrameMapAtlasGpu,
        ItemFrameMapDecorationAtlasGpu, ItemFrameMapDecorationSurface, ItemFrameMapSurface,
        ItemFrameMapTextFontAtlasGpu, ItemFrameMapTextSurface, ItemGlintTextureGpu, ItemModelMesh,
        SignTextSurface,
    },
    lightmap::{
        create_lightmap_bind_group_layout, create_lightmap_gpu, create_lightmap_pipeline,
        create_lightmap_sample_bind_group_layout, LightmapGpu,
    },
    outline::{
        create_entity_outline_bind_group_layout, create_entity_outline_blit_pipeline,
        create_entity_outline_blur_horizontal_pipeline,
        create_entity_outline_blur_vertical_pipeline, create_entity_outline_composite_pipeline,
        create_entity_outline_sobel_pipeline, create_entity_outline_target, EntityOutlineTarget,
    },
    particles::{
        create_particle_pipeline, ParticleAtlasGpu, ParticlePipelineKind, ParticleRuntimeState,
        ParticleUvRect,
    },
    player_skin::{DynamicPlayerSkinImage, DynamicPlayerTextureImage},
    selection::{
        create_selection_outline_gpu, create_selection_pipeline, SelectionOutline,
        SelectionOutlineGpu,
    },
    sky::{
        create_celestial_atlas_gpu, create_celestial_bind_group_layout, create_celestial_gpu,
        create_celestial_pipeline, create_end_sky_bind_group_layout, create_end_sky_gpu,
        create_end_sky_pipeline, create_end_sky_texture_gpu, create_sky_disc_gpu,
        create_sky_dynamic_bind_group_layout, create_sky_pipeline, create_star_gpu,
        create_star_pipeline, create_sunrise_sunset_pipeline, write_sky_model_view_dynamic,
        CelestialAtlasGpu, CelestialGpu, CelestialTextureImage, EndSkyGpu, EndSkyTextureGpu,
        SkyDiscGpu, SkyEnvironment, StarGpu,
    },
    terrain,
    transparency::{
        create_item_entity_target, create_main_target, create_particle_target,
        create_translucent_target, create_transparency_blit_bind_group_layout,
        create_transparency_blit_pipeline, create_transparency_combine_bind_group,
        create_transparency_combine_bind_group_layout, create_transparency_combine_pipeline,
        create_transparency_final_target, create_weather_target, ItemEntityTarget, MainTarget,
        ParticleTarget, TranslucentTarget, TransparencyCombineBindGroup, TransparencyFinalTarget,
        WeatherTarget,
    },
    weather::{
        create_lightning_pipeline, create_weather_pipeline, create_weather_texture_gpu,
        WeatherRenderState, WeatherTextureGpu, WeatherTextureImage, WeatherTextureKind,
    },
    world_border::{
        create_world_border_pipeline, create_world_border_texture_gpu, WorldBorderRenderState,
        WorldBorderTextureGpu,
    },
};

/// Where `render()` sends the frame's color output. Production always draws
/// into the window swapchain; whole-frame readback tests swap in a plain
/// offscreen texture so the full frame-step sequence runs without a window.
pub(super) enum RenderSurface {
    Window(wgpu::Surface<'static>),
    #[cfg(test)]
    Offscreen(std::sync::Arc<wgpu::Texture>),
}

impl RenderSurface {
    /// Acquires this frame's color target. `Ok(None)` means "skip this frame"
    /// — the swapchain was lost/outdated (reconfigured here, vanilla-style) or
    /// timed out — preserving the pre-harness `render()` surface semantics.
    pub(super) fn acquire_frame(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<Option<FrameTarget>> {
        match self {
            Self::Window(surface) => match surface.get_current_texture() {
                Ok(frame) => Ok(Some(FrameTarget::Surface(frame))),
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    surface.configure(device, config);
                    Ok(None)
                }
                Err(wgpu::SurfaceError::Timeout) => Ok(None),
                Err(err) => Err(err.into()),
            },
            #[cfg(test)]
            Self::Offscreen(texture) => Ok(Some(FrameTarget::Offscreen(texture.clone()))),
        }
    }

    pub(super) fn configure(&self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        match self {
            Self::Window(surface) => surface.configure(device, config),
            #[cfg(test)]
            Self::Offscreen(_) => {}
        }
    }
}

/// One frame's acquired color target: a swapchain frame to present, or the
/// offscreen texture (readback harness) where present is a no-op.
pub(super) enum FrameTarget {
    Surface(wgpu::SurfaceTexture),
    #[cfg(test)]
    Offscreen(std::sync::Arc<wgpu::Texture>),
}

impl FrameTarget {
    pub(super) fn texture(&self) -> &wgpu::Texture {
        match self {
            Self::Surface(frame) => &frame.texture,
            #[cfg(test)]
            Self::Offscreen(texture) => texture.as_ref(),
        }
    }

    pub(super) fn present(self) {
        match self {
            Self::Surface(frame) => frame.present(),
            #[cfg(test)]
            Self::Offscreen(_) => {}
        }
    }
}

pub struct Renderer {
    pub(super) surface: RenderSurface,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) size: PhysicalSize<u32>,
    pub(super) clear: ClearColor,
    pub(super) counters: RendererCounters,
    pub(super) started_at: Instant,
    pub(super) main_target: MainTarget,
    pub(super) translucent_target: TranslucentTarget,
    pub(super) item_entity_target: ItemEntityTarget,
    pub(super) particle_target: ParticleTarget,
    pub(super) weather_target: WeatherTarget,
    pub(super) transparency_combine_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) transparency_combine_bind_group: TransparencyCombineBindGroup,
    pub(super) transparency_combine_pipeline: wgpu::RenderPipeline,
    pub(super) transparency_blit_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) transparency_blit_pipeline: wgpu::RenderPipeline,
    pub(super) transparency_final_target: TransparencyFinalTarget,
    pub(super) depth: DepthTarget,
    pub(super) terrain_pipeline: wgpu::RenderPipeline,
    pub(super) terrain_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) block_destroy_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_textured_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_textured_cull_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_dissolve_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_cutout_z_offset_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_armor_cutout_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_translucent_cull_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_armor_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_translucent_emissive_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_eyes_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_dragon_rays_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_dragon_rays_depth_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_water_mask_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_outline_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_outline_cull_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_scroll_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_scroll_additive_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_entity_glint_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_armor_entity_glint_pipeline: wgpu::RenderPipeline,
    pub(super) opaque_particle_pipeline: wgpu::RenderPipeline,
    pub(super) translucent_particle_pipeline: wgpu::RenderPipeline,
    pub(super) frame_opaque_particle_vertices: FrameDataBuffer,
    pub(super) frame_translucent_particle_vertices: FrameDataBuffer,
    pub(super) frame_experience_orb_pickup_particle_vertices: FrameDataBuffer,
    pub(super) frame_experience_orb_pickup_particle_indices: FrameDataBuffer,
    pub(super) frame_projectile_pickup_particle_vertices: FrameDataBuffer,
    pub(super) frame_projectile_pickup_particle_indices: FrameDataBuffer,
    pub(super) frame_elder_guardian_particle_vertices: FrameDataBuffer,
    pub(super) frame_elder_guardian_particle_indices: FrameDataBuffer,
    pub(super) frame_weather_vertices: FrameDataBuffer,
    pub(super) frame_weather_indices: FrameDataBuffer,
    pub(super) frame_lightning_vertices: FrameDataBuffer,
    pub(super) frame_lightning_indices: FrameDataBuffer,
    pub(super) frame_world_border_vertices: FrameDataBuffer,
    pub(super) frame_world_border_indices: FrameDataBuffer,
    pub(super) frame_item_entity_vertices: FrameDataBuffer,
    pub(super) frame_hud_vertices: FrameDataBuffer,
    pub(super) weather_pipeline: wgpu::RenderPipeline,
    pub(super) lightning_pipeline: wgpu::RenderPipeline,
    pub(super) world_border_pipeline: wgpu::RenderPipeline,
    pub(super) item_entity_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_z_offset_forward_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_glint_pipeline: wgpu::RenderPipeline,
    pub(super) hud_item_glint_pipeline: wgpu::RenderPipeline,
    pub(super) selection_pipeline: wgpu::RenderPipeline,
    pub(super) lightmap_pipeline: wgpu::RenderPipeline,
    pub(super) lightmap: LightmapGpu,
    pub(super) entity_outline_sobel_pipeline: wgpu::RenderPipeline,
    pub(super) entity_outline_blur_horizontal_pipeline: wgpu::RenderPipeline,
    pub(super) entity_outline_blur_vertical_pipeline: wgpu::RenderPipeline,
    pub(super) entity_outline_blit_pipeline: wgpu::RenderPipeline,
    pub(super) entity_outline_composite_pipeline: wgpu::RenderPipeline,
    pub(super) entity_outline_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) entity_outline_target: EntityOutlineTarget,
    pub(super) sky_pipeline: wgpu::RenderPipeline,
    pub(super) sunrise_sunset_pipeline: wgpu::RenderPipeline,
    pub(super) star_pipeline: wgpu::RenderPipeline,
    pub(super) sky_dynamic_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) end_sky_pipeline: wgpu::RenderPipeline,
    pub(super) end_sky_texture_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) celestial_pipeline: wgpu::RenderPipeline,
    pub(super) celestial_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) cloud_flat_pipeline: wgpu::RenderPipeline,
    pub(super) cloud_fancy_pipeline: wgpu::RenderPipeline,
    pub(super) cloud_target: CloudTarget,
    pub(super) cloud_bind_group: wgpu::BindGroup,
    pub(super) cloud_uniform_buffer: wgpu::Buffer,
    pub(super) hud_pipeline: wgpu::RenderPipeline,
    pub(super) hud_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) hud_white_pixel: HudSpriteGpu,
    pub(super) hud_black_pixel: HudSpriteGpu,
    pub(super) terrain_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) camera_buffer: wgpu::Buffer,
    pub(super) gui_item_camera_buffer: wgpu::Buffer,
    pub(super) terrain_atlas: TerrainAtlasGpu,
    pub(super) terrain_bind_group: wgpu::BindGroup,
    pub(super) gui_item_bind_group: wgpu::BindGroup,
    pub(super) hud_hotbar_block_item_models: Vec<Option<crate::item_models::HudBlockItemModel>>,
    pub(super) terrain_opaque: Vec<ResidentTerrainMesh>,
    pub(super) terrain_cutout: Vec<ResidentTerrainMesh>,
    pub(super) terrain_translucent: Vec<ResidentTerrainMesh>,
    // Cross-section back-to-front draw order (far→near) of indices into
    // `terrain_translucent`. Vanilla draws the TRANSLUCENT layer reversed so
    // sections render far→near (ChunkSectionsToRender.java:55-56, MC 26.1); we
    // keep this order sorted by section bounding-box-center distance and refresh
    // it on every camera change / mesh upload. See
    // `rebuild_translucent_section_order`.
    pub(super) terrain_translucent_order: Vec<usize>,
    pub(super) terrain_source_sections: usize,
    pub(super) terrain_bounds: Option<TerrainBounds>,
    pub(super) entity_model_bounds: Option<TerrainBounds>,
    pub(super) camera_pose: Option<CameraPose>,
    pub(super) lightmap_environment: LightmapEnvironment,
    pub(super) fog_environment: FogEnvironment,
    pub(super) sky_environment: SkyEnvironment,
    pub(super) cloud_environment: CloudEnvironment,
    pub(super) cloud_frame: CloudFrame,
    pub(super) cloud_shape: CloudShape,
    pub(super) sky_disc: Option<SkyDiscGpu>,
    pub(super) end_sky_mesh: EndSkyGpu,
    pub(super) end_sky_texture: Option<EndSkyTextureGpu>,
    pub(super) sky_celestials: Option<CelestialGpu>,
    pub(super) sky_stars: Option<StarGpu>,
    pub(super) celestial_atlas: Option<CelestialAtlasGpu>,
    pub(super) cloud_texture: Option<CloudTextureData>,
    pub(super) clouds: Option<CloudGpu>,
    pub(super) block_destroy_overlays: Option<BlockDestroyOverlaysGpu>,
    pub(super) entity_model_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_water_mask_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_textured_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_textured_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_dissolve_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_cutout_z_offset_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_armor_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_armor_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_translucent_emissive_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_item_entity_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_item_entity_translucent_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_sorted_main_translucent_draws: Vec<EntityModelTranslucentDrawRange>,
    pub(super) entity_model_sorted_translucent_draws: Vec<EntityModelTexturedDrawRange>,
    pub(super) entity_model_sorted_item_entity_draws: Vec<EntityModelTexturedDrawRange>,
    pub(super) entity_model_eyes_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_dragon_rays_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_dragon_rays_depth_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_end_portal_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_end_gateway_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_outline_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_outline_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_entity_glint_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_armor_entity_glint_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_dynamic_player_skin_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_cutout_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_cutout_z_offset_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_item_entity_translucent_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_item_entity_translucent_cull_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_cutout_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_cutout_z_offset_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_armor_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_item_entity_translucent_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_item_entity_translucent_cull_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) first_person_player_arm_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) first_person_dynamic_player_arm_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_scroll_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_scroll_additive_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_texture_atlas: Option<EntityModelTextureAtlasGpu>,
    pub(super) entity_dynamic_player_skin_atlas: Option<EntityDynamicPlayerSkinAtlasGpu>,
    pub(super) entity_dynamic_player_skin_images: Vec<DynamicPlayerSkinImage>,
    pub(super) entity_dynamic_player_texture_atlas: Option<EntityDynamicPlayerTextureAtlasGpu>,
    pub(super) entity_dynamic_player_texture_images: Vec<DynamicPlayerTextureImage>,
    pub(super) entity_model_instances: Vec<crate::EntityModelInstance>,
    pub(super) first_person_player_arms: Vec<FirstPersonPlayerArm>,
    pub(super) particle_atlas: Option<ParticleAtlasGpu>,
    pub(super) terrain_particle_sprite_uvs: BTreeMap<String, ParticleUvRect>,
    pub(super) terrain_particle_translucent_sprites: BTreeSet<String>,
    pub(super) item_particle_sprite_uvs: BTreeMap<String, ParticleUvRect>,
    pub(super) item_particle_translucent_sprites: BTreeSet<String>,
    pub(super) weather_rain_texture: Option<WeatherTextureGpu>,
    pub(super) weather_snow_texture: Option<WeatherTextureGpu>,
    pub(super) weather_render_state: WeatherRenderState,
    pub(super) world_border_texture: Option<WorldBorderTextureGpu>,
    pub(super) world_border_render_state: WorldBorderRenderState,
    pub(super) item_entity_atlas: Option<ItemEntityAtlasGpu>,
    pub(super) item_glint_texture: Option<ItemGlintTextureGpu>,
    pub(super) item_entity_billboards: Vec<ItemEntityBillboard>,
    pub(super) block_item_model_meshes: Vec<ItemModelMesh>,
    pub(super) block_item_model_z_offset_forward_meshes: Vec<ItemModelMesh>,
    pub(super) block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(super) flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(super) item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) item_pickup_block_item_model_meshes: Vec<ItemModelMesh>,
    pub(super) item_pickup_block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) item_pickup_flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(super) item_pickup_flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) item_pickup_item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(super) item_pickup_item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) first_person_block_item_model_meshes: Vec<ItemModelMesh>,
    pub(super) first_person_block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) first_person_flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(super) first_person_flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) first_person_item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(super) first_person_item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(super) item_frame_map_surfaces: Vec<ItemFrameMapSurface>,
    pub(super) first_person_map_surfaces: Vec<ItemFrameMapSurface>,
    pub(super) first_person_map_background_surfaces: Vec<FirstPersonMapBackgroundSurface>,
    pub(super) first_person_map_background_atlas: Option<FirstPersonMapBackgroundAtlasGpu>,
    pub(super) item_frame_map_atlas: Option<ItemFrameMapAtlasGpu>,
    pub(super) item_frame_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub(super) first_person_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub(super) item_frame_map_decoration_atlas: Option<ItemFrameMapDecorationAtlasGpu>,
    pub(super) item_frame_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    pub(super) first_person_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    pub(super) sign_text_surfaces: Vec<SignTextSurface>,
    pub(super) item_frame_map_text_font_atlas: Option<ItemFrameMapTextFontAtlasGpu>,
    pub(super) selection_outline: Option<SelectionOutlineGpu>,
    pub(super) entity_scene_outline: Option<SelectionOutlineGpu>,
    pub(super) entity_target_outline: Option<SelectionOutlineGpu>,
    pub(super) hud_crosshair: Option<HudSpriteGpu>,
    pub(super) hud_hotbar: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_selection: Option<HudSpriteGpu>,
    pub(super) hud_item_atlas: Option<HudSpriteGpu>,
    pub(super) hud_digit_atlas: Option<HudSpriteGpu>,
    pub(super) hud_digit_glyphs: [HudDigitGlyph; 10],
    pub(super) hud_font_atlas: Option<HudSpriteGpu>,
    pub(super) hud_font_glyphs: HudFontGlyphMap,
    /// Advance-grouped glyph pool for obfuscated (`§k`) text, rebuilt once when
    /// `hud_font_glyphs` is uploaded so per-frame draws never rescan the table.
    pub(super) hud_obfuscated_glyph_pool: HudObfuscatedGlyphPool,
    pub(super) hud_hotbar_item_icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS],
    pub(super) hud_inventory_background: Option<HudSpriteGpu>,
    pub(super) hud_tooltip_background: Option<HudNineSliceSprite>,
    pub(super) hud_tooltip_frame: Option<HudNineSliceSprite>,
    pub(super) hud_generic_container_background: Option<HudSpriteGpu>,
    pub(super) hud_dispenser_background: Option<HudSpriteGpu>,
    pub(super) hud_crafting_table_background: Option<HudSpriteGpu>,
    pub(super) hud_cartography_table_background: Option<HudSpriteGpu>,
    pub(super) hud_cartography_table_error: Option<HudSpriteGpu>,
    pub(super) hud_cartography_table_scaled_map: Option<HudSpriteGpu>,
    pub(super) hud_cartography_table_duplicated_map: Option<HudSpriteGpu>,
    pub(super) hud_cartography_table_map: Option<HudSpriteGpu>,
    pub(super) hud_cartography_table_locked: Option<HudSpriteGpu>,
    pub(super) hud_loom_background: Option<HudSpriteGpu>,
    pub(super) hud_loom_banner_slot: Option<HudSpriteGpu>,
    pub(super) hud_loom_dye_slot: Option<HudSpriteGpu>,
    pub(super) hud_loom_pattern_slot: Option<HudSpriteGpu>,
    pub(super) hud_loom_scroller: Option<HudSpriteGpu>,
    pub(super) hud_loom_scroller_disabled: Option<HudSpriteGpu>,
    pub(super) hud_loom_pattern_selected: Option<HudSpriteGpu>,
    pub(super) hud_loom_pattern_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_loom_pattern: Option<HudSpriteGpu>,
    pub(super) hud_loom_error: Option<HudSpriteGpu>,
    pub(super) hud_crafter_background: Option<HudSpriteGpu>,
    pub(super) hud_crafter_disabled_slot: Option<HudSpriteGpu>,
    pub(super) hud_crafter_powered_redstone: Option<HudSpriteGpu>,
    pub(super) hud_crafter_unpowered_redstone: Option<HudSpriteGpu>,
    pub(super) hud_widget_text_field: Option<HudSpriteGpu>,
    pub(super) hud_widget_text_field_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_widget_button: Option<HudSpriteGpu>,
    pub(super) hud_widget_button_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_anvil_background: Option<HudSpriteGpu>,
    pub(super) hud_anvil_text_field: Option<HudSpriteGpu>,
    pub(super) hud_anvil_text_field_disabled: Option<HudSpriteGpu>,
    pub(super) hud_anvil_error: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_background: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_lapis_slot: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_enchantment_slot_disabled: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_enchantment_slot_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_enchantment_slot: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_level_1: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_level_2: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_level_3: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_level_1_disabled: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_level_2_disabled: Option<HudSpriteGpu>,
    pub(super) hud_enchanting_table_level_3_disabled: Option<HudSpriteGpu>,
    pub(super) hud_beacon_background: Option<HudSpriteGpu>,
    pub(super) hud_beacon_button_disabled: Option<HudSpriteGpu>,
    pub(super) hud_beacon_button_selected: Option<HudSpriteGpu>,
    pub(super) hud_beacon_button_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_beacon_button: Option<HudSpriteGpu>,
    pub(super) hud_beacon_confirm: Option<HudSpriteGpu>,
    pub(super) hud_beacon_cancel: Option<HudSpriteGpu>,
    pub(super) hud_beacon_effect_speed: Option<HudSpriteGpu>,
    pub(super) hud_beacon_effect_haste: Option<HudSpriteGpu>,
    pub(super) hud_beacon_effect_resistance: Option<HudSpriteGpu>,
    pub(super) hud_beacon_effect_jump_boost: Option<HudSpriteGpu>,
    pub(super) hud_beacon_effect_strength: Option<HudSpriteGpu>,
    pub(super) hud_beacon_effect_regeneration: Option<HudSpriteGpu>,
    pub(super) hud_brewing_stand_background: Option<HudSpriteGpu>,
    pub(super) hud_brewing_stand_fuel_length: Option<HudSpriteGpu>,
    pub(super) hud_brewing_stand_brew_progress: Option<HudSpriteGpu>,
    pub(super) hud_brewing_stand_bubbles: Option<HudSpriteGpu>,
    pub(super) hud_furnace_background: Option<HudSpriteGpu>,
    pub(super) hud_furnace_lit_progress: Option<HudSpriteGpu>,
    pub(super) hud_furnace_burn_progress: Option<HudSpriteGpu>,
    pub(super) hud_advancements_window: Option<HudSpriteGpu>,
    pub(super) hud_advancement_tabs: [Option<HudSpriteGpu>; HudAdvancementTabSprite::COUNT],
    pub(super) hud_advancement_backgrounds:
        [Option<HudSpriteGpu>; HudAdvancementBackgroundTexture::COUNT],
    pub(super) hud_advancement_widget_frames:
        [Option<HudSpriteGpu>; HudAdvancementWidgetFrameSprite::COUNT],
    pub(super) hud_advancement_hover_boxes:
        [Option<HudSpriteGpu>; HudAdvancementHoverBoxSprite::COUNT],
    pub(super) hud_recipe_book_background: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_tab: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_tab_selected: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_button: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_button_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_filter_enabled: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_filter_disabled: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_filter_enabled_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_filter_disabled_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_filter_enabled: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_filter_disabled: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_filter_enabled_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_filter_disabled_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_slot_craftable: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_slot_uncraftable: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_slot_many_craftable: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_slot_many_uncraftable: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_page_forward: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_page_forward_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_page_backward: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_page_backward_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_overlay_recipe: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_crafting_overlay: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_crafting_overlay_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_crafting_overlay_disabled: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_crafting_overlay_disabled_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_overlay: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_overlay_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_overlay_disabled: Option<HudSpriteGpu>,
    pub(super) hud_recipe_book_furnace_overlay_disabled_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_blast_furnace_background: Option<HudSpriteGpu>,
    pub(super) hud_blast_furnace_lit_progress: Option<HudSpriteGpu>,
    pub(super) hud_blast_furnace_burn_progress: Option<HudSpriteGpu>,
    pub(super) hud_smoker_background: Option<HudSpriteGpu>,
    pub(super) hud_smoker_lit_progress: Option<HudSpriteGpu>,
    pub(super) hud_smoker_burn_progress: Option<HudSpriteGpu>,
    pub(super) hud_smithing_background: Option<HudSpriteGpu>,
    pub(super) hud_smithing_error: Option<HudSpriteGpu>,
    pub(super) hud_grindstone_background: Option<HudSpriteGpu>,
    pub(super) hud_grindstone_error: Option<HudSpriteGpu>,
    pub(super) hud_hopper_background: Option<HudSpriteGpu>,
    pub(super) hud_horse_background: Option<HudSpriteGpu>,
    pub(super) hud_nautilus_background: Option<HudSpriteGpu>,
    pub(super) hud_mount_slot: Option<HudSpriteGpu>,
    pub(super) hud_mount_saddle_slot: Option<HudSpriteGpu>,
    pub(super) hud_mount_horse_armor_slot: Option<HudSpriteGpu>,
    pub(super) hud_mount_llama_armor_slot: Option<HudSpriteGpu>,
    pub(super) hud_mount_nautilus_armor_slot: Option<HudSpriteGpu>,
    pub(super) hud_mount_chest_slots: Option<HudSpriteGpu>,
    pub(super) hud_book_background: Option<HudSpriteGpu>,
    pub(super) hud_page_backward: Option<HudSpriteGpu>,
    pub(super) hud_page_forward: Option<HudSpriteGpu>,
    pub(super) hud_hanging_sign_backgrounds: [Option<HudSpriteGpu>; 12],
    pub(super) hud_shulker_box_background: Option<HudSpriteGpu>,
    pub(super) hud_stonecutter_background: Option<HudSpriteGpu>,
    pub(super) hud_stonecutter_scroller: Option<HudSpriteGpu>,
    pub(super) hud_stonecutter_scroller_disabled: Option<HudSpriteGpu>,
    pub(super) hud_stonecutter_recipe_selected: Option<HudSpriteGpu>,
    pub(super) hud_stonecutter_recipe_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_stonecutter_recipe: Option<HudSpriteGpu>,
    pub(super) hud_villager_background: Option<HudSpriteGpu>,
    pub(super) hud_villager_out_of_stock: Option<HudSpriteGpu>,
    pub(super) hud_villager_experience_bar_background: Option<HudSpriteGpu>,
    pub(super) hud_villager_experience_bar_current: Option<HudSpriteGpu>,
    pub(super) hud_villager_experience_bar_result: Option<HudSpriteGpu>,
    pub(super) hud_villager_scroller: Option<HudSpriteGpu>,
    pub(super) hud_villager_scroller_disabled: Option<HudSpriteGpu>,
    pub(super) hud_villager_trade_arrow: Option<HudSpriteGpu>,
    pub(super) hud_villager_trade_arrow_out_of_stock: Option<HudSpriteGpu>,
    pub(super) hud_villager_discount_strikethrough: Option<HudSpriteGpu>,
    pub(super) hud_slot_highlight_back: Option<HudSpriteGpu>,
    pub(super) hud_slot_highlight_front: Option<HudSpriteGpu>,
    pub(super) hud_inventory_screen: Option<HudInventoryScreen>,
    pub(super) hud_sign_editor_screen: Option<crate::hud::HudSignEditorScreen>,
    /// One persistent PIP target per sanitized `hud_inventory_screen` entity preview, indexed by
    /// preview order (vanilla `PictureInPictureRenderer` private color+depth textures).
    pub(super) hud_entity_preview_pip_targets: Vec<HudEntityPreviewPipTarget>,
    pub(super) hud_experience_background: Option<HudSpriteGpu>,
    pub(super) hud_experience_progress: Option<HudSpriteGpu>,
    pub(super) hud_jump_bar_background: Option<HudSpriteGpu>,
    pub(super) hud_jump_bar_progress: Option<HudSpriteGpu>,
    pub(super) hud_jump_bar_cooldown: Option<HudSpriteGpu>,
    /// Player heart sprites keyed by `[HudHeartKind as usize][variant]`, where
    /// `variant = (hardcore as usize) * 2 + (half as usize)` (non-blinking
    /// variants only; blink is deferred). `Container`'s half slots mirror its
    /// full slot (vanilla routes both to the container sprite).
    pub(super) hud_heart_sprites: [[Option<HudSpriteGpu>; 4]; crate::hud::HudHeartKind::ALL.len()],
    pub(super) hud_food_empty: Option<HudSpriteGpu>,
    pub(super) hud_food_full: Option<HudSpriteGpu>,
    pub(super) hud_food_half: Option<HudSpriteGpu>,
    pub(super) hud_food_empty_hunger: Option<HudSpriteGpu>,
    pub(super) hud_food_full_hunger: Option<HudSpriteGpu>,
    pub(super) hud_food_half_hunger: Option<HudSpriteGpu>,
    pub(super) hud_armor_empty: Option<HudSpriteGpu>,
    pub(super) hud_armor_half: Option<HudSpriteGpu>,
    pub(super) hud_armor_full: Option<HudSpriteGpu>,
    pub(super) hud_air_bubble: Option<HudSpriteGpu>,
    pub(super) hud_air_bubble_bursting: Option<HudSpriteGpu>,
    pub(super) hud_air_bubble_empty: Option<HudSpriteGpu>,
    pub(super) hud_heart_vehicle_container: Option<HudSpriteGpu>,
    pub(super) hud_heart_vehicle_full: Option<HudSpriteGpu>,
    pub(super) hud_heart_vehicle_half: Option<HudSpriteGpu>,
    pub(super) hud_code_of_conduct_overlay: Option<HudSpriteGpu>,
    pub(super) hud_player_health: Option<crate::hud::HudPlayerHealth>,
    pub(super) hud_food: Option<i32>,
    pub(super) hud_food_effect: crate::hud::HudFoodEffect,
    pub(super) hud_armor: Option<i32>,
    pub(super) hud_air: Option<crate::hud::HudAirSupply>,
    pub(super) hud_vehicle_health: Option<crate::hud::HudVehicleHealth>,
    pub(super) hud_jump_bar: Option<crate::hud::HudJumpBar>,
    pub(super) hud_experience_progress_value: Option<f32>,
    pub(super) hud_experience_level: Option<i32>,
    pub(super) hud_selected_slot: u8,
    pub(super) hud_action_bar_text: Option<crate::hud::HudActionBarText>,
    pub(super) hud_title_text: Option<crate::hud::HudTitleText>,
    /// This frame's projected boss bars plus the 182x5 sprite sheets (one
    /// texture per vanilla `boss_bar/*` GUI sprite, indexed by the enum
    /// ordinals like `BossHealthOverlay`'s sprite arrays).
    pub(super) hud_boss_bars: Vec<crate::hud::HudBossBar>,
    pub(super) hud_boss_bar_backgrounds:
        [Option<HudSpriteGpu>; crate::hud::HudBossBarColor::ALL.len()],
    pub(super) hud_boss_bar_progress_sprites:
        [Option<HudSpriteGpu>; crate::hud::HudBossBarColor::ALL.len()],
    pub(super) hud_boss_bar_notched_backgrounds:
        [Option<HudSpriteGpu>; crate::hud::HudBossBarOverlay::NOTCHED.len()],
    pub(super) hud_boss_bar_notched_progress_sprites:
        [Option<HudSpriteGpu>; crate::hud::HudBossBarOverlay::NOTCHED.len()],
    pub(super) particles: ParticleRuntimeState,
}

pub(super) struct ResidentTerrainMesh {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) vertex_count: usize,
    pub(super) index_count: usize,
    pub(super) opaque_faces: usize,
    pub(super) cutout_faces: usize,
    pub(super) translucent_faces: usize,
    pub(super) culled_faces: usize,
    pub(super) resident_bytes: u64,
    // Axis-aligned bounds of this section's translucent geometry; its center is
    // the sort key for the cross-section back-to-front draw order. `None` only
    // for degenerate (empty-vertex) meshes, which are filtered before residency.
    bounds: Option<TerrainBounds>,
    translucent_sort: Option<TerrainTranslucentSortState>,
}

#[derive(Debug, Clone)]
struct TerrainTranslucentSortState {
    centroids: Vec<[f32; 3]>,
    last_camera_position: Option<[f32; 3]>,
}

impl TerrainTranslucentSortState {
    fn from_vertices(
        vertices: &[terrain::TerrainVertex],
        last_camera_position: Option<[f32; 3]>,
    ) -> Option<Self> {
        if vertices.len() % 4 != 0 {
            return None;
        }

        let centroids = vertices
            .chunks_exact(4)
            .map(|quad| {
                [
                    (quad[0].position[0] + quad[2].position[0]) * 0.5,
                    (quad[0].position[1] + quad[2].position[1]) * 0.5,
                    (quad[0].position[2] + quad[2].position[2]) * 0.5,
                ]
            })
            .collect();
        Some(Self {
            centroids,
            last_camera_position,
        })
    }

    fn indices_for_camera_if_changed(&mut self, camera_position: [f32; 3]) -> Option<Vec<u32>> {
        if self.last_camera_position == Some(camera_position) {
            return None;
        }
        self.last_camera_position = Some(camera_position);
        Some(self.sorted_indices(camera_position))
    }

    fn sorted_indices(&self, camera_position: [f32; 3]) -> Vec<u32> {
        let mut quad_distances: Vec<_> = self
            .centroids
            .iter()
            .enumerate()
            .map(|(quad_index, centroid)| {
                let dx = centroid[0] - camera_position[0];
                let dy = centroid[1] - camera_position[1];
                let dz = centroid[2] - camera_position[2];
                (quad_index, dx * dx + dy * dy + dz * dz)
            })
            .collect();
        quad_distances.sort_by(
            |(left_index, left_distance), (right_index, right_distance)| {
                right_distance
                    .total_cmp(left_distance)
                    .then_with(|| left_index.cmp(right_index))
            },
        );

        let mut indices = Vec::with_capacity(quad_distances.len() * 6);
        for (quad_index, _) in quad_distances {
            let base = (quad_index * 4) as u32;
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
        }
        indices
    }
}

/// Back-to-front (far→near) draw order for translucent terrain *sections*.
///
/// Vanilla `ChunkSectionsToRender.renderGroup` accumulates draws for every layer
/// in the near→far `visibleSections` BFS order (LevelRenderer.java:1063-1134, MC
/// 26.1) but reverses that list for the TRANSLUCENT layer only
/// (ChunkSectionsToRender.java:55-56: `draws = draws.reversed()`), so translucent
/// sections composite from the farthest to the nearest. We reproduce that by
/// sorting resident translucent section meshes by the squared distance from each
/// section's bounding-box center to the camera sort position, descending — the
/// same distance basis and tie-break shape as the within-section quad resort
/// (`TerrainTranslucentSortState::sorted_indices`). Sections whose bounds are
/// absent (degenerate meshes) sink to the end, and equal distances keep ascending
/// section index so the order is stable and jitter-free frame-to-frame.
fn translucent_section_draw_order(
    centers: &[Option<[f32; 3]>],
    camera_position: [f32; 3],
) -> Vec<usize> {
    let distances: Vec<f32> = centers
        .iter()
        .map(|center| {
            center
                .map(|center| {
                    let dx = center[0] - camera_position[0];
                    let dy = center[1] - camera_position[1];
                    let dz = center[2] - camera_position[2];
                    dx * dx + dy * dy + dz * dz
                })
                .unwrap_or(f32::NEG_INFINITY)
        })
        .collect();
    let mut order: Vec<usize> = (0..centers.len()).collect();
    order.sort_by(|&left, &right| {
        distances[right]
            .total_cmp(&distances[left])
            .then_with(|| left.cmp(&right))
    });
    order
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(window)?)?
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("no suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("bbb-native-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = choose_format(&caps.formats)?;
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes
                .first()
                .copied()
                .ok_or_else(|| anyhow!("surface has no present modes"))?
        };
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        Self::with_gpu(RenderSurface::Window(surface), device, queue, config, size)
    }

    /// Builds the renderer around an already-acquired GPU and frame surface.
    /// `new` calls this with the window swapchain; the offscreen readback
    /// harness calls it with an offscreen texture target, so both paths share
    /// one construction body.
    pub(super) fn with_gpu(
        surface: RenderSurface,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        size: PhysicalSize<u32>,
    ) -> Result<Self> {
        let format = config.format;
        let main_target = create_main_target(&device, config.format, config.width, config.height);
        let depth = create_depth_target(&device, config.width, config.height);
        let translucent_target =
            create_translucent_target(&device, config.format, config.width, config.height);
        let item_entity_target =
            create_item_entity_target(&device, config.format, config.width, config.height);
        let particle_target =
            create_particle_target(&device, config.format, config.width, config.height);
        let weather_target =
            create_weather_target(&device, config.format, config.width, config.height);
        let terrain_bind_group_layout = create_terrain_bind_group_layout(&device);
        let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
        let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
        let lightmap_pipeline = create_lightmap_pipeline(&device, &lightmap_bind_group_layout);
        let lightmap = create_lightmap_gpu(
            &device,
            &queue,
            &lightmap_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            LightmapEnvironment::default(),
        );
        let hud_bind_group_layout = create_hud_bind_group_layout(&device);
        let camera_buffer = create_camera_buffer(&device);
        let gui_item_camera_buffer = create_camera_buffer(&device);
        let terrain_atlas = create_terrain_atlas_gpu(&device, &queue, 1, 1, &[255, 255, 255, 255])?;
        let terrain_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &camera_buffer,
            &terrain_atlas,
        );
        // The GUI item pass reuses the item-model pipeline + blocks atlas but with its own ortho camera.
        let gui_item_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &gui_item_camera_buffer,
            &terrain_atlas,
        );
        let terrain_pipeline = create_terrain_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let terrain_translucent_pipeline = create_terrain_translucent_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let block_destroy_pipeline =
            create_block_destroy_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_pipeline = create_entity_model_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_textured_pipeline = create_entity_model_textured_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_textured_cull_pipeline = create_entity_model_textured_cull_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_dissolve_pipeline = create_entity_model_dissolve_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_cutout_z_offset_pipeline = create_entity_model_cutout_z_offset_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_armor_cutout_pipeline = create_entity_model_armor_cutout_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_translucent_pipeline = create_entity_model_translucent_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_translucent_cull_pipeline = create_entity_model_translucent_cull_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_armor_translucent_pipeline =
            create_entity_model_armor_translucent_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
            );
        let entity_model_translucent_emissive_pipeline =
            create_entity_model_translucent_emissive_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
            );
        let entity_model_eyes_pipeline =
            create_entity_model_eyes_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_dragon_rays_pipeline =
            create_entity_model_dragon_rays_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_dragon_rays_depth_pipeline =
            create_entity_model_dragon_rays_depth_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
            );
        let entity_model_water_mask_pipeline =
            create_entity_model_water_mask_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_outline_pipeline =
            create_entity_model_outline_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_outline_cull_pipeline =
            create_entity_model_outline_cull_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_scroll_pipeline = create_entity_model_scroll_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let entity_model_scroll_additive_pipeline = create_entity_model_scroll_additive_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
        );
        let entity_model_entity_glint_pipeline =
            create_entity_model_entity_glint_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_armor_entity_glint_pipeline =
            create_entity_model_armor_entity_glint_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
            );
        let opaque_particle_pipeline = create_particle_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            ParticlePipelineKind::Opaque,
        );
        let translucent_particle_pipeline = create_particle_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            ParticlePipelineKind::Translucent,
        );
        let weather_pipeline = create_weather_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let lightning_pipeline =
            create_lightning_pipeline(&device, format, &terrain_bind_group_layout);
        let world_border_pipeline =
            create_world_border_pipeline(&device, format, &terrain_bind_group_layout);
        let item_entity_pipeline = create_item_entity_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let item_model_pipeline = create_item_model_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let item_model_z_offset_forward_pipeline = create_item_model_z_offset_forward_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let item_model_translucent_pipeline = create_item_model_translucent_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let item_model_glint_pipeline =
            create_item_model_glint_pipeline(&device, format, &terrain_bind_group_layout);
        let hud_item_glint_pipeline = create_hud_item_glint_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &hud_bind_group_layout,
        );
        let selection_pipeline =
            create_selection_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_outline_bind_group_layout = create_entity_outline_bind_group_layout(&device);
        let entity_outline_sobel_pipeline = create_entity_outline_sobel_pipeline(
            &device,
            format,
            &entity_outline_bind_group_layout,
        );
        let entity_outline_blur_horizontal_pipeline =
            create_entity_outline_blur_horizontal_pipeline(
                &device,
                format,
                &entity_outline_bind_group_layout,
            );
        let entity_outline_blur_vertical_pipeline = create_entity_outline_blur_vertical_pipeline(
            &device,
            format,
            &entity_outline_bind_group_layout,
        );
        let entity_outline_blit_pipeline =
            create_entity_outline_blit_pipeline(&device, format, &entity_outline_bind_group_layout);
        let entity_outline_composite_pipeline = create_entity_outline_composite_pipeline(
            &device,
            format,
            &entity_outline_bind_group_layout,
        );
        let entity_outline_target = create_entity_outline_target(
            &device,
            &entity_outline_bind_group_layout,
            format,
            config.width,
            config.height,
        );
        let sky_dynamic_bind_group_layout = create_sky_dynamic_bind_group_layout(&device);
        let sky_pipeline = create_sky_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &sky_dynamic_bind_group_layout,
        );
        let sunrise_sunset_pipeline =
            create_sunrise_sunset_pipeline(&device, format, &terrain_bind_group_layout);
        let star_pipeline = create_star_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &sky_dynamic_bind_group_layout,
        );
        let end_sky_texture_bind_group_layout = create_end_sky_bind_group_layout(&device);
        let end_sky_pipeline = create_end_sky_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &end_sky_texture_bind_group_layout,
            &sky_dynamic_bind_group_layout,
        );
        let end_sky_mesh = create_end_sky_gpu(&device, &sky_dynamic_bind_group_layout);
        let celestial_bind_group_layout = create_celestial_bind_group_layout(&device);
        let celestial_pipeline = create_celestial_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &celestial_bind_group_layout,
            &sky_dynamic_bind_group_layout,
        );
        let cloud_bind_group_layout = create_cloud_bind_group_layout(&device);
        let cloud_uniform_buffer = create_cloud_uniform_buffer(&device);
        let cloud_bind_group =
            create_cloud_bind_group(&device, &cloud_bind_group_layout, &cloud_uniform_buffer);
        let cloud_target = create_cloud_target(&device, format, config.width, config.height);
        let transparency_combine_bind_group_layout =
            create_transparency_combine_bind_group_layout(&device);
        let transparency_combine_bind_group = create_transparency_combine_bind_group(
            &device,
            &transparency_combine_bind_group_layout,
            &main_target,
            &depth,
            &translucent_target,
            &item_entity_target,
            &particle_target,
            &weather_target,
            &cloud_target,
        );
        let transparency_combine_pipeline = create_transparency_combine_pipeline(
            &device,
            format,
            &transparency_combine_bind_group_layout,
        );
        let transparency_blit_bind_group_layout =
            create_transparency_blit_bind_group_layout(&device);
        let transparency_final_target = create_transparency_final_target(
            &device,
            &transparency_blit_bind_group_layout,
            format,
            config.width,
            config.height,
        );
        let transparency_blit_pipeline = create_transparency_blit_pipeline(
            &device,
            format,
            &transparency_blit_bind_group_layout,
        );
        let cloud_flat_pipeline = create_cloud_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &cloud_bind_group_layout,
            CloudShape::Flat,
        );
        let cloud_fancy_pipeline = create_cloud_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &cloud_bind_group_layout,
            CloudShape::Fancy,
        );
        let hud_pipeline = create_hud_pipeline(&device, format, &hud_bind_group_layout);
        let hud_white_pixel = create_hud_sprite_gpu(
            &device,
            &queue,
            &hud_bind_group_layout,
            1,
            1,
            &[255, 255, 255, 255],
        )?;
        let hud_black_pixel = create_hud_sprite_gpu(
            &device,
            &queue,
            &hud_bind_group_layout,
            1,
            1,
            &[0, 0, 0, 255],
        )?;

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            clear: ClearColor::default(),
            counters: RendererCounters {
                width: size.width,
                height: size.height,
                ..RendererCounters::default()
            },
            started_at: Instant::now(),
            main_target,
            translucent_target,
            item_entity_target,
            particle_target,
            weather_target,
            transparency_combine_bind_group_layout,
            transparency_combine_bind_group,
            transparency_combine_pipeline,
            transparency_blit_bind_group_layout,
            transparency_blit_pipeline,
            transparency_final_target,
            depth,
            terrain_pipeline,
            terrain_translucent_pipeline,
            block_destroy_pipeline,
            entity_model_pipeline,
            entity_model_textured_pipeline,
            entity_model_textured_cull_pipeline,
            entity_model_dissolve_pipeline,
            entity_model_cutout_z_offset_pipeline,
            entity_model_armor_cutout_pipeline,
            entity_model_translucent_pipeline,
            entity_model_translucent_cull_pipeline,
            entity_model_armor_translucent_pipeline,
            entity_model_translucent_emissive_pipeline,
            entity_model_eyes_pipeline,
            entity_model_dragon_rays_pipeline,
            entity_model_dragon_rays_depth_pipeline,
            entity_model_water_mask_pipeline,
            entity_model_outline_pipeline,
            entity_model_outline_cull_pipeline,
            entity_model_scroll_pipeline,
            entity_model_scroll_additive_pipeline,
            entity_model_entity_glint_pipeline,
            entity_model_armor_entity_glint_pipeline,
            opaque_particle_pipeline,
            translucent_particle_pipeline,
            frame_opaque_particle_vertices: FrameDataBuffer::vertex(
                "bbb-opaque-particle-frame-vertices",
            ),
            frame_translucent_particle_vertices: FrameDataBuffer::vertex(
                "bbb-translucent-particle-frame-vertices",
            ),
            frame_experience_orb_pickup_particle_vertices: FrameDataBuffer::vertex(
                "bbb-experience-orb-pickup-particle-frame-vertices",
            ),
            frame_experience_orb_pickup_particle_indices: FrameDataBuffer::index(
                "bbb-experience-orb-pickup-particle-frame-indices",
            ),
            frame_projectile_pickup_particle_vertices: FrameDataBuffer::vertex(
                "bbb-projectile-pickup-particle-frame-vertices",
            ),
            frame_projectile_pickup_particle_indices: FrameDataBuffer::index(
                "bbb-projectile-pickup-particle-frame-indices",
            ),
            frame_elder_guardian_particle_vertices: FrameDataBuffer::vertex(
                "bbb-elder-guardian-particle-frame-vertices",
            ),
            frame_elder_guardian_particle_indices: FrameDataBuffer::index(
                "bbb-elder-guardian-particle-frame-indices",
            ),
            frame_weather_vertices: FrameDataBuffer::vertex("bbb-weather-frame-vertices"),
            frame_weather_indices: FrameDataBuffer::index("bbb-weather-frame-indices"),
            frame_lightning_vertices: FrameDataBuffer::vertex("bbb-lightning-frame-vertices"),
            frame_lightning_indices: FrameDataBuffer::index("bbb-lightning-frame-indices"),
            frame_world_border_vertices: FrameDataBuffer::vertex("bbb-world-border-frame-vertices"),
            frame_world_border_indices: FrameDataBuffer::index("bbb-world-border-frame-indices"),
            frame_item_entity_vertices: FrameDataBuffer::vertex("bbb-item-entity-frame-vertices"),
            frame_hud_vertices: FrameDataBuffer::vertex("bbb-hud-frame-vertices"),
            weather_pipeline,
            lightning_pipeline,
            world_border_pipeline,
            item_entity_pipeline,
            item_model_pipeline,
            item_model_z_offset_forward_pipeline,
            item_model_translucent_pipeline,
            item_model_glint_pipeline,
            hud_item_glint_pipeline,
            selection_pipeline,
            lightmap_pipeline,
            lightmap,
            entity_outline_sobel_pipeline,
            entity_outline_blur_horizontal_pipeline,
            entity_outline_blur_vertical_pipeline,
            entity_outline_blit_pipeline,
            entity_outline_composite_pipeline,
            entity_outline_bind_group_layout,
            entity_outline_target,
            sky_pipeline,
            sunrise_sunset_pipeline,
            star_pipeline,
            sky_dynamic_bind_group_layout,
            end_sky_pipeline,
            end_sky_texture_bind_group_layout,
            celestial_pipeline,
            celestial_bind_group_layout,
            cloud_flat_pipeline,
            cloud_fancy_pipeline,
            cloud_target,
            cloud_bind_group,
            cloud_uniform_buffer,
            hud_pipeline,
            hud_bind_group_layout,
            hud_white_pixel,
            hud_black_pixel,
            terrain_bind_group_layout,
            camera_buffer,
            gui_item_camera_buffer,
            terrain_atlas,
            terrain_bind_group,
            gui_item_bind_group,
            hud_hotbar_block_item_models: Vec::new(),
            terrain_opaque: Vec::new(),
            terrain_cutout: Vec::new(),
            terrain_translucent: Vec::new(),
            terrain_translucent_order: Vec::new(),
            terrain_source_sections: 0,
            terrain_bounds: None,
            entity_model_bounds: None,
            camera_pose: None,
            lightmap_environment: LightmapEnvironment::default(),
            fog_environment: FogEnvironment::default(),
            sky_environment: SkyEnvironment::default(),
            cloud_environment: CloudEnvironment::default(),
            cloud_frame: CloudFrame::default(),
            cloud_shape: CloudShape::default(),
            sky_disc: None,
            end_sky_mesh,
            end_sky_texture: None,
            sky_celestials: None,
            sky_stars: None,
            celestial_atlas: None,
            cloud_texture: None,
            clouds: None,
            block_destroy_overlays: None,
            entity_model_mesh: None,
            entity_model_water_mask_mesh: None,
            entity_model_textured_mesh: None,
            entity_model_textured_cull_mesh: None,
            entity_model_dissolve_mesh: None,
            entity_model_cutout_z_offset_mesh: None,
            entity_model_armor_cutout_mesh: None,
            entity_model_translucent_mesh: None,
            entity_model_armor_translucent_mesh: None,
            entity_model_translucent_emissive_mesh: None,
            entity_model_item_entity_translucent_mesh: None,
            entity_model_item_entity_translucent_cull_mesh: None,
            entity_model_sorted_main_translucent_draws: Vec::new(),
            entity_model_sorted_translucent_draws: Vec::new(),
            entity_model_sorted_item_entity_draws: Vec::new(),
            entity_model_eyes_mesh: None,
            entity_model_dragon_rays_mesh: None,
            entity_model_dragon_rays_depth_mesh: None,
            entity_model_end_portal_mesh: None,
            entity_model_end_gateway_mesh: None,
            entity_model_outline_mesh: None,
            entity_model_outline_cull_mesh: None,
            entity_model_entity_glint_mesh: None,
            entity_model_armor_entity_glint_mesh: None,
            entity_dynamic_player_skin_cutout_mesh: None,
            entity_dynamic_player_skin_cutout_cull_mesh: None,
            entity_dynamic_player_skin_cutout_z_offset_mesh: None,
            entity_dynamic_player_skin_translucent_mesh: None,
            entity_dynamic_player_skin_item_entity_translucent_mesh: None,
            entity_dynamic_player_skin_item_entity_translucent_cull_mesh: None,
            entity_dynamic_player_texture_cutout_mesh: None,
            entity_dynamic_player_texture_cutout_cull_mesh: None,
            entity_dynamic_player_texture_cutout_z_offset_mesh: None,
            entity_dynamic_player_texture_armor_cutout_mesh: None,
            entity_dynamic_player_texture_translucent_mesh: None,
            entity_dynamic_player_texture_item_entity_translucent_mesh: None,
            entity_dynamic_player_texture_item_entity_translucent_cull_mesh: None,
            first_person_player_arm_mesh: None,
            first_person_dynamic_player_arm_mesh: None,
            entity_model_scroll_mesh: None,
            entity_model_scroll_additive_mesh: None,
            entity_model_texture_atlas: None,
            entity_dynamic_player_skin_atlas: None,
            entity_dynamic_player_skin_images: Vec::new(),
            entity_dynamic_player_texture_atlas: None,
            entity_dynamic_player_texture_images: Vec::new(),
            entity_model_instances: Vec::new(),
            first_person_player_arms: Vec::new(),
            particle_atlas: None,
            terrain_particle_sprite_uvs: BTreeMap::new(),
            terrain_particle_translucent_sprites: BTreeSet::new(),
            item_particle_sprite_uvs: BTreeMap::new(),
            item_particle_translucent_sprites: BTreeSet::new(),
            weather_rain_texture: None,
            weather_snow_texture: None,
            weather_render_state: WeatherRenderState::default(),
            world_border_texture: None,
            world_border_render_state: WorldBorderRenderState::default(),
            item_entity_atlas: None,
            item_glint_texture: None,
            item_entity_billboards: Vec::new(),
            block_item_model_meshes: Vec::new(),
            block_item_model_z_offset_forward_meshes: Vec::new(),
            block_item_model_translucent_meshes: Vec::new(),
            flat_item_model_meshes: Vec::new(),
            flat_item_model_translucent_meshes: Vec::new(),
            item_model_glint_meshes: Vec::new(),
            item_model_glint_translucent_meshes: Vec::new(),
            item_pickup_block_item_model_meshes: Vec::new(),
            item_pickup_block_item_model_translucent_meshes: Vec::new(),
            item_pickup_flat_item_model_meshes: Vec::new(),
            item_pickup_flat_item_model_translucent_meshes: Vec::new(),
            item_pickup_item_model_glint_meshes: Vec::new(),
            item_pickup_item_model_glint_translucent_meshes: Vec::new(),
            first_person_block_item_model_meshes: Vec::new(),
            first_person_block_item_model_translucent_meshes: Vec::new(),
            first_person_flat_item_model_meshes: Vec::new(),
            first_person_flat_item_model_translucent_meshes: Vec::new(),
            first_person_item_model_glint_meshes: Vec::new(),
            first_person_item_model_glint_translucent_meshes: Vec::new(),
            item_frame_map_surfaces: Vec::new(),
            first_person_map_surfaces: Vec::new(),
            first_person_map_background_surfaces: Vec::new(),
            first_person_map_background_atlas: None,
            item_frame_map_atlas: None,
            item_frame_map_decoration_surfaces: Vec::new(),
            first_person_map_decoration_surfaces: Vec::new(),
            item_frame_map_decoration_atlas: None,
            item_frame_map_text_surfaces: Vec::new(),
            first_person_map_text_surfaces: Vec::new(),
            sign_text_surfaces: Vec::new(),
            item_frame_map_text_font_atlas: None,
            selection_outline: None,
            entity_scene_outline: None,
            entity_target_outline: None,
            hud_crosshair: None,
            hud_hotbar: None,
            hud_hotbar_selection: None,
            hud_item_atlas: None,
            hud_digit_atlas: None,
            hud_digit_glyphs: [HudDigitGlyph::default(); 10],
            hud_font_atlas: None,
            hud_font_glyphs: HudFontGlyphMap::default(),
            hud_obfuscated_glyph_pool: HudObfuscatedGlyphPool::default(),
            hud_hotbar_item_icons: std::array::from_fn(|_| None),
            hud_inventory_background: None,
            hud_tooltip_background: None,
            hud_tooltip_frame: None,
            hud_generic_container_background: None,
            hud_dispenser_background: None,
            hud_crafting_table_background: None,
            hud_cartography_table_background: None,
            hud_cartography_table_error: None,
            hud_cartography_table_scaled_map: None,
            hud_cartography_table_duplicated_map: None,
            hud_cartography_table_map: None,
            hud_cartography_table_locked: None,
            hud_loom_background: None,
            hud_loom_banner_slot: None,
            hud_loom_dye_slot: None,
            hud_loom_pattern_slot: None,
            hud_loom_scroller: None,
            hud_loom_scroller_disabled: None,
            hud_loom_pattern_selected: None,
            hud_loom_pattern_highlighted: None,
            hud_loom_pattern: None,
            hud_loom_error: None,
            hud_crafter_background: None,
            hud_crafter_disabled_slot: None,
            hud_crafter_powered_redstone: None,
            hud_crafter_unpowered_redstone: None,
            hud_widget_text_field: None,
            hud_widget_text_field_highlighted: None,
            hud_widget_button: None,
            hud_widget_button_highlighted: None,
            hud_anvil_background: None,
            hud_anvil_text_field: None,
            hud_anvil_text_field_disabled: None,
            hud_anvil_error: None,
            hud_enchanting_table_background: None,
            hud_enchanting_table_lapis_slot: None,
            hud_enchanting_table_enchantment_slot_disabled: None,
            hud_enchanting_table_enchantment_slot_highlighted: None,
            hud_enchanting_table_enchantment_slot: None,
            hud_enchanting_table_level_1: None,
            hud_enchanting_table_level_2: None,
            hud_enchanting_table_level_3: None,
            hud_enchanting_table_level_1_disabled: None,
            hud_enchanting_table_level_2_disabled: None,
            hud_enchanting_table_level_3_disabled: None,
            hud_beacon_background: None,
            hud_beacon_button_disabled: None,
            hud_beacon_button_selected: None,
            hud_beacon_button_highlighted: None,
            hud_beacon_button: None,
            hud_beacon_confirm: None,
            hud_beacon_cancel: None,
            hud_beacon_effect_speed: None,
            hud_beacon_effect_haste: None,
            hud_beacon_effect_resistance: None,
            hud_beacon_effect_jump_boost: None,
            hud_beacon_effect_strength: None,
            hud_beacon_effect_regeneration: None,
            hud_brewing_stand_background: None,
            hud_brewing_stand_fuel_length: None,
            hud_brewing_stand_brew_progress: None,
            hud_brewing_stand_bubbles: None,
            hud_furnace_background: None,
            hud_furnace_lit_progress: None,
            hud_furnace_burn_progress: None,
            hud_advancements_window: None,
            hud_advancement_tabs: std::array::from_fn(|_| None),
            hud_advancement_backgrounds: std::array::from_fn(|_| None),
            hud_advancement_widget_frames: std::array::from_fn(|_| None),
            hud_advancement_hover_boxes: std::array::from_fn(|_| None),
            hud_recipe_book_background: None,
            hud_recipe_book_tab: None,
            hud_recipe_book_tab_selected: None,
            hud_recipe_book_button: None,
            hud_recipe_book_button_highlighted: None,
            hud_recipe_book_filter_enabled: None,
            hud_recipe_book_filter_disabled: None,
            hud_recipe_book_filter_enabled_highlighted: None,
            hud_recipe_book_filter_disabled_highlighted: None,
            hud_recipe_book_furnace_filter_enabled: None,
            hud_recipe_book_furnace_filter_disabled: None,
            hud_recipe_book_furnace_filter_enabled_highlighted: None,
            hud_recipe_book_furnace_filter_disabled_highlighted: None,
            hud_recipe_book_slot_craftable: None,
            hud_recipe_book_slot_uncraftable: None,
            hud_recipe_book_slot_many_craftable: None,
            hud_recipe_book_slot_many_uncraftable: None,
            hud_recipe_book_page_forward: None,
            hud_recipe_book_page_forward_highlighted: None,
            hud_recipe_book_page_backward: None,
            hud_recipe_book_page_backward_highlighted: None,
            hud_recipe_book_overlay_recipe: None,
            hud_recipe_book_crafting_overlay: None,
            hud_recipe_book_crafting_overlay_highlighted: None,
            hud_recipe_book_crafting_overlay_disabled: None,
            hud_recipe_book_crafting_overlay_disabled_highlighted: None,
            hud_recipe_book_furnace_overlay: None,
            hud_recipe_book_furnace_overlay_highlighted: None,
            hud_recipe_book_furnace_overlay_disabled: None,
            hud_recipe_book_furnace_overlay_disabled_highlighted: None,
            hud_blast_furnace_background: None,
            hud_blast_furnace_lit_progress: None,
            hud_blast_furnace_burn_progress: None,
            hud_smoker_background: None,
            hud_smoker_lit_progress: None,
            hud_smoker_burn_progress: None,
            hud_smithing_background: None,
            hud_smithing_error: None,
            hud_grindstone_background: None,
            hud_grindstone_error: None,
            hud_hopper_background: None,
            hud_horse_background: None,
            hud_nautilus_background: None,
            hud_mount_slot: None,
            hud_mount_saddle_slot: None,
            hud_mount_horse_armor_slot: None,
            hud_mount_llama_armor_slot: None,
            hud_mount_nautilus_armor_slot: None,
            hud_mount_chest_slots: None,
            hud_book_background: None,
            hud_page_backward: None,
            hud_page_forward: None,
            hud_hanging_sign_backgrounds: std::array::from_fn(|_| None),
            hud_shulker_box_background: None,
            hud_stonecutter_background: None,
            hud_stonecutter_scroller: None,
            hud_stonecutter_scroller_disabled: None,
            hud_stonecutter_recipe_selected: None,
            hud_stonecutter_recipe_highlighted: None,
            hud_stonecutter_recipe: None,
            hud_villager_background: None,
            hud_villager_out_of_stock: None,
            hud_villager_experience_bar_background: None,
            hud_villager_experience_bar_current: None,
            hud_villager_experience_bar_result: None,
            hud_villager_scroller: None,
            hud_villager_scroller_disabled: None,
            hud_villager_trade_arrow: None,
            hud_villager_trade_arrow_out_of_stock: None,
            hud_villager_discount_strikethrough: None,
            hud_slot_highlight_back: None,
            hud_slot_highlight_front: None,
            hud_inventory_screen: None,
            hud_sign_editor_screen: None,
            hud_entity_preview_pip_targets: Vec::new(),
            hud_experience_background: None,
            hud_experience_progress: None,
            hud_jump_bar_background: None,
            hud_jump_bar_progress: None,
            hud_jump_bar_cooldown: None,
            hud_heart_sprites: Default::default(),
            hud_food_empty: None,
            hud_food_full: None,
            hud_food_half: None,
            hud_food_empty_hunger: None,
            hud_food_full_hunger: None,
            hud_food_half_hunger: None,
            hud_armor_empty: None,
            hud_armor_half: None,
            hud_armor_full: None,
            hud_air_bubble: None,
            hud_air_bubble_bursting: None,
            hud_air_bubble_empty: None,
            hud_heart_vehicle_container: None,
            hud_heart_vehicle_full: None,
            hud_heart_vehicle_half: None,
            hud_code_of_conduct_overlay: None,
            hud_player_health: None,
            hud_food: None,
            hud_food_effect: crate::hud::HudFoodEffect::default(),
            hud_armor: None,
            hud_air: None,
            hud_vehicle_health: None,
            hud_jump_bar: None,
            hud_experience_progress_value: None,
            hud_experience_level: None,
            hud_selected_slot: 0,
            hud_action_bar_text: None,
            hud_title_text: None,
            hud_boss_bars: Vec::new(),
            hud_boss_bar_backgrounds: Default::default(),
            hud_boss_bar_progress_sprites: Default::default(),
            hud_boss_bar_notched_backgrounds: Default::default(),
            hud_boss_bar_notched_progress_sprites: Default::default(),
            particles: ParticleRuntimeState::default(),
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.main_target = create_main_target(
            &self.device,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.depth = create_depth_target(&self.device, self.config.width, self.config.height);
        self.translucent_target = create_translucent_target(
            &self.device,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.item_entity_target = create_item_entity_target(
            &self.device,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.particle_target = create_particle_target(
            &self.device,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.weather_target = create_weather_target(
            &self.device,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.transparency_final_target = create_transparency_final_target(
            &self.device,
            &self.transparency_blit_bind_group_layout,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.entity_outline_target = create_entity_outline_target(
            &self.device,
            &self.entity_outline_bind_group_layout,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.cloud_target = create_cloud_target(
            &self.device,
            self.config.format,
            self.config.width,
            self.config.height,
        );
        self.transparency_combine_bind_group = create_transparency_combine_bind_group(
            &self.device,
            &self.transparency_combine_bind_group_layout,
            &self.main_target,
            &self.depth,
            &self.translucent_target,
            &self.item_entity_target,
            &self.particle_target,
            &self.weather_target,
            &self.cloud_target,
        );
        self.update_camera();
        self.counters.width = size.width;
        self.counters.height = size.height;
    }

    pub fn counters(&self) -> RendererCounters {
        self.counters.clone()
    }

    pub fn set_camera_pose(&mut self, pose: Option<CameraPose>) {
        if self.camera_pose == pose {
            return;
        }
        self.camera_pose = pose;
        self.resort_translucent_terrain_for_camera();
        if self.entity_model_texture_atlas.is_some() && !self.entity_model_instances.is_empty() {
            self.rebuild_entity_model_meshes();
        }
        self.update_camera();
    }

    pub fn set_clear_color(&mut self, clear: ClearColor) {
        self.clear = clear;
    }

    pub fn set_selection_outline(&mut self, outline: Option<SelectionOutline>) {
        if self
            .selection_outline
            .as_ref()
            .map(|selection| &selection.outline)
            == outline.as_ref()
        {
            return;
        }
        self.selection_outline =
            outline.map(|outline| create_selection_outline_gpu(&self.device, outline));
    }

    pub fn set_entity_scene_outline(&mut self, outline: Option<SelectionOutline>) {
        let entity_scene_boxes = outline.as_ref().map_or(0, |outline| outline.boxes.len());
        self.counters.entity_scene_boxes = entity_scene_boxes;
        if self
            .entity_scene_outline
            .as_ref()
            .map(|selection| &selection.outline)
            == outline.as_ref()
        {
            return;
        }
        self.entity_scene_outline =
            outline.map(|outline| create_selection_outline_gpu(&self.device, outline));
    }

    pub fn set_entity_target_outline(&mut self, outline: Option<SelectionOutline>) {
        if self
            .entity_target_outline
            .as_ref()
            .map(|selection| &selection.outline)
            == outline.as_ref()
        {
            return;
        }
        self.entity_target_outline =
            outline.map(|outline| create_selection_outline_gpu(&self.device, outline));
    }

    pub fn set_block_destroy_overlays(&mut self, overlays: Vec<BlockDestroyOverlay>) {
        let overlays = if overlays.is_empty() {
            None
        } else {
            Some(overlays)
        };
        if self
            .block_destroy_overlays
            .as_ref()
            .map(|resident| resident.overlays.as_slice())
            == overlays.as_deref()
        {
            return;
        }
        self.block_destroy_overlays =
            overlays.map(|overlays| create_block_destroy_overlays_gpu(&self.device, overlays));
    }

    pub fn upload_terrain_meshes(&mut self, meshes: Vec<terrain::TerrainMesh>) {
        let source_sections = meshes.iter().map(|mesh| mesh.source_sections).sum();
        self.upload_terrain_mesh_layers(terrain::TerrainMeshLayers {
            opaque: meshes,
            cutout: Vec::new(),
            translucent: Vec::new(),
            source_sections,
        });
    }

    pub fn upload_terrain_mesh_layers(&mut self, layers: terrain::TerrainMeshLayers) {
        self.terrain_source_sections = layers.source_sections;
        self.counters.queued_sections = self.terrain_source_sections;
        self.counters.meshed_sections = self.terrain_source_sections;
        self.counters.upload_bytes = 0;
        self.terrain_opaque.clear();
        self.terrain_cutout.clear();
        self.terrain_translucent.clear();
        self.terrain_bounds = None;

        for mesh in layers.opaque {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_opaque.push(resident);
            }
        }
        for mesh in layers.cutout {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_cutout.push(resident);
            }
        }
        for mesh in layers.translucent {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_translucent.push(resident);
            }
        }
        // The translucent section set just changed (full replace is the only
        // add/remove path); refresh the far→near draw order so it stays a valid,
        // camera-correct permutation of `terrain_translucent`.
        self.rebuild_translucent_section_order();

        self.update_camera();
        self.refresh_terrain_counters();
    }

    pub fn upload_terrain_texture_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.upload_terrain_texture_atlas_mips(width, height, &[rgba])
    }

    pub fn upload_terrain_texture_atlas_mips(
        &mut self,
        width: u32,
        height: u32,
        mip_rgba: &[&[u8]],
    ) -> Result<()> {
        self.terrain_atlas =
            create_terrain_atlas_mips_gpu(&self.device, &self.queue, width, height, mip_rgba)?;
        self.terrain_bind_group = create_terrain_bind_group(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            &self.terrain_atlas,
        );
        self.gui_item_bind_group = create_terrain_bind_group(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.gui_item_camera_buffer,
            &self.terrain_atlas,
        );
        self.counters.atlas_pages = 1;
        self.counters.atlas_reallocations += 1;
        self.counters.atlas_width = width;
        self.counters.atlas_height = height;
        Ok(())
    }

    pub fn update_terrain_texture_atlas(&mut self, rgba: &[u8]) -> Result<()> {
        write_terrain_atlas_gpu(&self.queue, &self.terrain_atlas, rgba)
    }

    pub fn update_terrain_texture_atlas_mips(&mut self, mip_rgba: &[&[u8]]) -> Result<()> {
        write_terrain_atlas_mips_gpu(&self.queue, &self.terrain_atlas, mip_rgba)
    }

    pub(super) fn surface_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.config.width.max(1), self.config.height.max(1))
    }

    fn refresh_terrain_counters(&mut self) {
        self.counters.uploaded_sections = if self.terrain_opaque.is_empty()
            && self.terrain_cutout.is_empty()
            && self.terrain_translucent.is_empty()
        {
            0
        } else {
            self.terrain_source_sections
        };
        self.counters.visible_sections = self.counters.uploaded_sections;
        self.counters.resident_bytes = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.resident_bytes)
            .sum();
        self.counters.terrain_vertices = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.vertex_count)
            .sum();
        self.counters.terrain_indices = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.index_count)
            .sum();
        self.counters.opaque_faces = self
            .terrain_opaque
            .iter()
            .map(|mesh| mesh.opaque_faces)
            .sum();
        self.counters.cutout_faces = self
            .terrain_cutout
            .iter()
            .map(|mesh| mesh.cutout_faces)
            .sum();
        self.counters.translucent_faces = self
            .terrain_translucent
            .iter()
            .map(|mesh| mesh.translucent_faces)
            .sum();
        self.counters.culled_faces = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.culled_faces)
            .sum();
    }

    fn create_resident_terrain_mesh(
        &mut self,
        mesh: terrain::TerrainMesh,
    ) -> Option<ResidentTerrainMesh> {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return None;
        }

        let bounds = TerrainBounds::from_vertices(&mesh.vertices);
        let translucent_sort = (mesh.translucent_faces > 0)
            .then(|| {
                TerrainTranslucentSortState::from_vertices(
                    &mesh.vertices,
                    self.camera_sort_position(),
                )
            })
            .flatten();
        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes = bytemuck::cast_slice(&mesh.indices);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-terrain-vertex-buffer"),
                contents: vertex_bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-terrain-index-buffer"),
                contents: index_bytes,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
        let resident_bytes = (vertex_bytes.len() + index_bytes.len()) as u64;
        self.counters.upload_bytes += resident_bytes;
        if let Some(bounds) = bounds {
            if let Some(existing) = &mut self.terrain_bounds {
                existing.include_bounds(bounds);
            } else {
                self.terrain_bounds = Some(bounds);
            }
        }
        Some(ResidentTerrainMesh {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len(),
            index_count: mesh.indices.len(),
            opaque_faces: mesh.opaque_faces,
            cutout_faces: mesh.cutout_faces,
            translucent_faces: mesh.translucent_faces,
            culled_faces: mesh.culled_faces,
            resident_bytes,
            bounds,
            translucent_sort,
        })
    }

    pub(crate) fn camera_sort_position(&self) -> Option<[f32; 3]> {
        self.camera_pose.map(|pose| {
            [
                pose.position[0],
                pose.position[1] + pose.eye_height,
                pose.position[2],
            ]
        })
    }

    /// Recompute the cross-section back-to-front (far→near) draw order for the
    /// translucent terrain layer. Kept in lock-step with `terrain_translucent`
    /// (rebuilt on every mesh upload and every camera change) so the draw pass can
    /// iterate `terrain_translucent_order` as valid indices. With no camera pose
    /// yet the natural upload order is retained.
    fn rebuild_translucent_section_order(&mut self) {
        self.terrain_translucent_order = match self.camera_sort_position() {
            Some(camera_position) => {
                let centers: Vec<Option<[f32; 3]>> = self
                    .terrain_translucent
                    .iter()
                    .map(|mesh| mesh.bounds.map(|bounds| bounds.center().to_array()))
                    .collect();
                translucent_section_draw_order(&centers, camera_position)
            }
            None => (0..self.terrain_translucent.len()).collect(),
        };
    }

    fn resort_translucent_terrain_for_camera(&mut self) {
        // Segment (cross-section) order tracks the camera every frame, mirroring
        // vanilla's per-frame `draws.reversed()` for the TRANSLUCENT layer
        // (ChunkSectionsToRender.java:55-56, MC 26.1).
        self.rebuild_translucent_section_order();

        let Some(camera_position) = self.camera_sort_position() else {
            return;
        };
        let queue = &self.queue;
        for mesh in &mut self.terrain_translucent {
            let Some(sort) = &mut mesh.translucent_sort else {
                continue;
            };
            let Some(indices) = sort.indices_for_camera_if_changed(camera_position) else {
                continue;
            };
            queue.write_buffer(&mesh.index_buffer, 0, bytemuck::cast_slice(&indices));
        }
    }

    pub(crate) fn update_camera(&self) {
        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        let glint_elapsed_millis = self.started_at.elapsed().as_secs_f64() * 1000.0;
        let uniform = if let Some(pose) = self.camera_pose {
            CameraUniform::from_pose(pose, aspect)
        } else {
            self.scene_bounds()
                .map(|bounds| CameraUniform::from_bounds(bounds, aspect))
                .unwrap_or_else(CameraUniform::identity)
        }
        .with_lightmap_environment(self.lightmap_environment)
        .with_fog_environment(self.fog_environment)
        .with_glint_texture_time(glint_elapsed_millis, VANILLA_DEFAULT_GLINT_SPEED);
        self.update_sky_model_view_dynamics(uniform.sky_model_view_transform());
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
        // The GUI item pass projects 3D inventory icons with a screen-space ortho (separate buffer so it
        // does not clobber the world camera, which earlier passes in the same submit still read).
        let gui = CameraUniform::gui_ortho(self.config.width as f32, self.config.height as f32)
            .with_glint_texture_time(glint_elapsed_millis, VANILLA_DEFAULT_GLINT_SPEED);
        self.queue
            .write_buffer(&self.gui_item_camera_buffer, 0, bytemuck::bytes_of(&gui));
    }

    fn update_sky_model_view_dynamics(&self, sky_model_view: Mat4) {
        write_sky_model_view_dynamic(&self.queue, &self.end_sky_mesh.dynamic, sky_model_view);
        if let Some(sky_disc) = &self.sky_disc {
            write_sky_model_view_dynamic(&self.queue, &sky_disc.dynamic, sky_model_view);
        }
        if let Some(stars) = &self.sky_stars {
            write_sky_model_view_dynamic(&self.queue, &stars.dynamic, sky_model_view);
        }
        if let Some(celestials) = &self.sky_celestials {
            write_sky_model_view_dynamic(&self.queue, &celestials.sun.dynamic, sky_model_view);
            write_sky_model_view_dynamic(&self.queue, &celestials.moon.dynamic, sky_model_view);
        }
    }

    pub fn set_lightmap_brightness_factor(&mut self, factor: f32) {
        self.lightmap_environment.brightness_factor = sanitize_lightmap_brightness_factor(factor);
        self.update_camera();
    }

    pub fn set_lightmap_block_factor(&mut self, factor: f32) {
        self.lightmap_environment.block_factor = sanitize_lightmap_block_factor(factor);
        self.update_camera();
    }

    pub fn set_lightmap_environment(&mut self, environment: LightmapEnvironment) {
        self.lightmap_environment = environment.sanitized();
        self.update_camera();
    }

    pub fn set_fog_environment(&mut self, environment: FogEnvironment) {
        self.fog_environment = environment.sanitized();
        self.update_camera();
    }

    pub fn set_sky_environment(&mut self, environment: SkyEnvironment) {
        let environment = environment.sanitized();
        if self.sky_environment == environment {
            return;
        }
        self.sky_environment = environment;
        self.sky_disc = create_sky_disc_gpu(
            &self.device,
            &self.sky_dynamic_bind_group_layout,
            environment,
        );
        self.sky_celestials = self.celestial_atlas.as_ref().and_then(|atlas| {
            create_celestial_gpu(
                &self.device,
                &self.sky_dynamic_bind_group_layout,
                environment,
                atlas,
            )
        });
        self.sky_stars = create_star_gpu(
            &self.device,
            &self.sky_dynamic_bind_group_layout,
            environment,
        );
        self.update_camera();
    }

    pub fn set_cloud_environment(&mut self, environment: CloudEnvironment) {
        let environment = environment.sanitized();
        if self.cloud_environment == environment {
            return;
        }
        self.cloud_environment = environment;
        self.rebuild_clouds();
    }

    pub fn set_cloud_frame(&mut self, frame: CloudFrame) {
        let frame = frame.sanitized();
        if self.cloud_frame == frame {
            return;
        }
        let old_mesh_key = self.clouds.as_ref().and_then(|clouds| clouds.mesh_key);
        self.cloud_frame = frame;
        if cloud_mesh_key(
            self.cloud_environment,
            self.cloud_texture.as_ref(),
            frame,
            self.cloud_shape,
        ) != old_mesh_key
        {
            self.rebuild_clouds();
        } else {
            self.write_cloud_uniform();
        }
    }

    pub fn set_cloud_shape(&mut self, shape: CloudShape) {
        if self.cloud_shape == shape {
            return;
        }
        self.cloud_shape = shape;
        self.rebuild_clouds();
    }

    pub fn upload_end_sky_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.end_sky_texture = Some(create_end_sky_texture_gpu(
            &self.device,
            &self.queue,
            &self.end_sky_texture_bind_group_layout,
            width,
            height,
            rgba,
        )?);
        Ok(())
    }

    pub fn upload_celestial_textures(&mut self, images: &[CelestialTextureImage]) -> Result<()> {
        self.celestial_atlas = Some(create_celestial_atlas_gpu(
            &self.device,
            &self.queue,
            &self.celestial_bind_group_layout,
            images,
        )?);
        self.sky_celestials = self.celestial_atlas.as_ref().and_then(|atlas| {
            create_celestial_gpu(
                &self.device,
                &self.sky_dynamic_bind_group_layout,
                self.sky_environment,
                atlas,
            )
        });
        self.update_camera();
        Ok(())
    }

    pub fn upload_cloud_texture(&mut self, image: &CloudTextureImage) -> Result<()> {
        self.cloud_texture = Some(create_cloud_texture_data(image)?);
        self.rebuild_clouds();
        Ok(())
    }

    pub fn upload_weather_textures(&mut self, images: &[WeatherTextureImage]) -> Result<()> {
        for image in images {
            let texture = create_weather_texture_gpu(
                &self.device,
                &self.queue,
                &self.terrain_bind_group_layout,
                &self.camera_buffer,
                image,
            )?;
            match image.kind {
                WeatherTextureKind::Rain => self.weather_rain_texture = Some(texture),
                WeatherTextureKind::Snow => self.weather_snow_texture = Some(texture),
            }
        }
        Ok(())
    }

    pub fn set_weather_render_state(&mut self, state: WeatherRenderState) {
        self.weather_render_state = state;
    }

    /// Uploads the vanilla `textures/misc/forcefield.png` bytes
    /// (`WorldBorderRenderer.FORCEFIELD_LOCATION`, `WorldBorderRenderer.java:34`);
    /// the bytes are fed from the native side so this crate stays pack-free.
    pub fn upload_world_border_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.world_border_texture = Some(create_world_border_texture_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            width,
            height,
            rgba,
        )?);
        Ok(())
    }

    pub fn set_world_border_render_state(&mut self, state: WorldBorderRenderState) {
        self.world_border_render_state = state;
    }

    fn rebuild_clouds(&mut self) {
        self.clouds = create_cloud_gpu(
            &self.device,
            self.cloud_environment,
            self.cloud_texture.as_ref(),
            self.cloud_frame,
            self.cloud_shape,
        );
        self.write_cloud_uniform();
    }

    fn write_cloud_uniform(&self) {
        write_cloud_uniform(
            &self.queue,
            &self.cloud_uniform_buffer,
            self.cloud_frame,
            self.cloud_texture.as_ref(),
        );
    }

    fn scene_bounds(&self) -> Option<TerrainBounds> {
        let mut bounds = self.terrain_bounds.or(self.entity_model_bounds)?;
        if let Some(entity_model_bounds) = self.entity_model_bounds {
            bounds.include_bounds(entity_model_bounds);
        }
        Some(bounds)
    }
}

fn choose_format(formats: &[wgpu::TextureFormat]) -> Result<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| {
            matches!(
                f,
                wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb
            )
        })
        .or_else(|| {
            formats.iter().copied().find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
        })
        .ok_or_else(|| anyhow!("surface does not expose an RGBA/BGRA 8-bit format"))
}

#[cfg(test)]
mod tests {
    use super::{choose_format, translucent_section_draw_order, TerrainTranslucentSortState};
    use crate::camera::TerrainBounds;
    use crate::terrain::TerrainVertex;
    use glam::Vec3;

    #[test]
    fn choose_format_prefers_srgb_surface_formats_for_screenshot_readback() {
        assert_eq!(
            choose_format(&[
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureFormat::Rgba16Float,
            ])
            .unwrap(),
            wgpu::TextureFormat::Bgra8UnormSrgb
        );
        assert_eq!(
            choose_format(&[
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ])
            .unwrap(),
            wgpu::TextureFormat::Rgba8UnormSrgb
        );
    }

    #[test]
    fn choose_format_falls_back_to_unorm_screenshot_readback_formats() {
        assert_eq!(
            choose_format(&[
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureFormat::Bgra8Unorm,
            ])
            .unwrap(),
            wgpu::TextureFormat::Bgra8Unorm
        );
        assert_eq!(
            choose_format(&[
                wgpu::TextureFormat::Depth24Plus,
                wgpu::TextureFormat::Rgba8Unorm,
            ])
            .unwrap(),
            wgpu::TextureFormat::Rgba8Unorm
        );
    }

    #[test]
    fn choose_format_rejects_non_screenshot_readback_formats() {
        let err = choose_format(&[
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Depth24Plus,
        ])
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("surface does not expose an RGBA/BGRA 8-bit format"));
    }

    #[test]
    fn translucent_sort_state_rebuilds_vanilla_indices_after_camera_move() {
        let vertices = vec![
            terrain_vertex([0.0, 0.0, 0.0]),
            terrain_vertex([1.0, 0.0, 0.0]),
            terrain_vertex([1.0, 1.0, 0.0]),
            terrain_vertex([0.0, 1.0, 0.0]),
            terrain_vertex([0.0, 0.0, 1.0]),
            terrain_vertex([1.0, 0.0, 1.0]),
            terrain_vertex([1.0, 1.0, 1.0]),
            terrain_vertex([0.0, 1.0, 1.0]),
        ];
        let mut sort =
            TerrainTranslucentSortState::from_vertices(&vertices, Some([0.5, 0.5, -4.0]))
                .expect("quad vertices create sort state");

        assert_eq!(sort.indices_for_camera_if_changed([0.5, 0.5, -4.0]), None);
        assert_eq!(
            sort.indices_for_camera_if_changed([0.5, 0.5, 4.0]),
            Some(vec![0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4])
        );
        assert_eq!(
            sort.indices_for_camera_if_changed([0.5, 0.5, -4.0]),
            Some(vec![4, 5, 6, 6, 7, 4, 0, 1, 2, 2, 3, 0])
        );
    }

    fn terrain_vertex(position: [f32; 3]) -> TerrainVertex {
        TerrainVertex {
            position,
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            light: [0.0, 0.0],
            tint: [1.0, 1.0, 1.0],
            shade: 1.0,
            ambient_occlusion: 1.0,
            block_state_id: 0,
        }
    }

    /// Center of an axis-aligned bounding box, derived the same way the renderer
    /// derives a translucent section's sort key (`mesh.bounds.map(|b| b.center())`).
    fn section_center(min: [f32; 3], max: [f32; 3]) -> Option<[f32; 3]> {
        let bounds = TerrainBounds::from_points([Vec3::from_array(min), Vec3::from_array(max)])
            .expect("two corner points form bounds");
        Some(bounds.center().to_array())
    }

    #[test]
    fn translucent_sections_draw_back_to_front() {
        // Three 16³ translucent sections strung out along -Z with centers at
        // z = 0, -10, -20, indexed in storage order 0, 1, 2.
        let centers = [
            section_center([-8.0, -8.0, -8.0], [8.0, 8.0, 8.0]),
            section_center([-8.0, -8.0, -18.0], [8.0, 8.0, -2.0]),
            section_center([-8.0, -8.0, -28.0], [8.0, 8.0, -12.0]),
        ];

        // Camera in front (+Z) looking down -Z: farthest section (2) draws first,
        // nearest (0) last — vanilla reversed TRANSLUCENT order.
        let order = translucent_section_draw_order(&centers, [0.0, 0.0, 5.0]);
        assert_eq!(order, vec![2, 1, 0]);
    }

    #[test]
    fn translucent_section_order_reorders_after_camera_move() {
        let centers = [
            section_center([-8.0, -8.0, -8.0], [8.0, 8.0, 8.0]),
            section_center([-8.0, -8.0, -18.0], [8.0, 8.0, -2.0]),
            section_center([-8.0, -8.0, -28.0], [8.0, 8.0, -12.0]),
        ];

        // From +Z the far→near order is 2,1,0 ...
        assert_eq!(
            translucent_section_draw_order(&centers, [0.0, 0.0, 5.0]),
            vec![2, 1, 0]
        );
        // ... and moving the camera behind section 2 flips it to 0,1,2, proving
        // the segment order tracks the camera every rebuild rather than staying
        // pinned to storage order.
        assert_eq!(
            translucent_section_draw_order(&centers, [0.0, 0.0, -25.0]),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn translucent_section_order_is_stable_on_equal_distance() {
        // Three sections equidistant (100) from the camera at the origin: the tie
        // must break on ascending storage index so the order never jitters between
        // frames when distances collide.
        let centers = [
            section_center([2.0, -8.0, -8.0], [18.0, 8.0, 8.0]), // center x = 10
            section_center([-18.0, -8.0, -8.0], [-2.0, 8.0, 8.0]), // center x = -10
            section_center([-8.0, 2.0, -8.0], [8.0, 18.0, 8.0]), // center y = 10
        ];
        assert_eq!(
            translucent_section_draw_order(&centers, [0.0, 0.0, 0.0]),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn translucent_section_order_sinks_boundless_sections_to_the_end() {
        // A degenerate (boundless) section keeps a valid slot but sorts last so it
        // never occludes a real far→near draw.
        let centers = [
            section_center([-8.0, -8.0, -28.0], [8.0, 8.0, -12.0]), // farthest
            None,
            section_center([-8.0, -8.0, -8.0], [8.0, 8.0, 8.0]), // nearest
        ];
        let order = translucent_section_draw_order(&centers, [0.0, 0.0, 5.0]);
        assert_eq!(order, vec![0, 2, 1]);
    }
}
