use std::{
    collections::{hash_map::Entry, BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hasher},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use bbb_render_types::{EntityDefaultPlayerSkin, StandaloneTextureSampling};
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
        sanitize_lightmap_block_factor, sanitize_lightmap_brightness_factor, CameraHurtEffect,
        CameraPose, CameraSpinningEffect, CameraUniform, CameraViewBobEffect, ClearColor,
        FogEnvironment, LightmapEnvironment, TerrainBounds, VANILLA_DEFAULT_GLINT_SPEED,
    },
    camera_post_effect::{CameraPostEffectGpu, CameraPostEffectResize},
    clouds::{
        cloud_mesh_key, create_cloud_bind_group, create_cloud_bind_group_layout, create_cloud_gpu,
        create_cloud_pipeline, create_cloud_target, create_cloud_texture_data,
        create_cloud_uniform_buffer, write_cloud_uniform, CloudEnvironment, CloudFrame, CloudGpu,
        CloudShape, CloudTarget, CloudTextureData, CloudTextureImage,
    },
    counters::{RendererCounters, TerrainMeshWorkerCounters},
    debug_filled_boxes::{
        create_debug_filled_box_pipeline, sanitize_debug_filled_boxes, sanitize_debug_filled_rects,
        DebugFilledBox, DebugFilledRect,
    },
    debug_text_billboards::{
        create_debug_text_pipeline, sanitize_debug_text_billboards, DebugTextBillboard,
    },
    entity_models::{
        create_entity_model_armor_cutout_pipeline, create_entity_model_armor_decal_pipeline,
        create_entity_model_armor_entity_glint_pipeline,
        create_entity_model_armor_translucent_pipeline,
        create_entity_model_banner_pattern_pipeline,
        create_entity_model_beacon_beam_opaque_pipeline,
        create_entity_model_beacon_beam_translucent_pipeline,
        create_entity_model_cutout_z_offset_pipeline, create_entity_model_dissolve_pipeline,
        create_entity_model_dragon_rays_depth_pipeline, create_entity_model_dragon_rays_pipeline,
        create_entity_model_end_crystal_beam_pipeline, create_entity_model_end_gateway_pipeline,
        create_entity_model_end_portal_pipeline, create_entity_model_entity_glint_pipeline,
        create_entity_model_eyes_pipeline, create_entity_model_guardian_beam_pipeline,
        create_entity_model_leash_pipeline, create_entity_model_outline_cull_pipeline,
        create_entity_model_outline_pipeline, create_entity_model_pipeline,
        create_entity_model_scroll_additive_pipeline, create_entity_model_scroll_pipeline,
        create_entity_model_solid_pipeline, create_entity_model_solid_z_offset_forward_pipeline,
        create_entity_model_textured_cull_pipeline, create_entity_model_textured_pipeline,
        create_entity_model_translucent_cull_pipeline,
        create_entity_model_translucent_emissive_pipeline,
        create_entity_model_translucent_pipeline, create_entity_model_water_mask_pipeline,
        create_win_screen_end_portal_pipeline, EntityDynamicPlayerSkinAtlasGpu,
        EntityDynamicPlayerTextureAtlasGpu, EntityFlameSurface, EntityModelMeshGpu,
        EntityModelPortalMeshGpu, EntityModelScrollDrawRange, EntityModelScrollMeshGpu,
        EntityModelTextureAtlasGpu, EntityModelTexturedDrawRange, EntityModelTexturedMeshGpu,
        EntityModelTranslucentDrawRange, EntityShadowSurface, EntityShadowTextureGpu,
        FirstPersonPlayerArm, HudEntityPreviewPipTarget,
    },
    gpu::{
        create_camera_buffer, create_depth_target, create_terrain_atlas_gpu,
        create_terrain_atlas_mips_gpu, create_terrain_bind_group, create_terrain_bind_group_layout,
        create_terrain_cutout_pipeline, create_terrain_section_bind_group_layout,
        create_terrain_section_uniform, create_terrain_solid_pipeline,
        create_terrain_translucent_pipeline, write_terrain_atlas_gpu,
        write_terrain_atlas_mip_patches_gpu, write_terrain_atlas_mips_gpu, DepthTarget,
        TerrainAtlasGpu,
    },
    gpu_timing::{optional_gpu_timing_features, GpuFrameTiming},
    hud::{
        create_hud_bind_group_layout, create_hud_item_glint_pipeline,
        create_hud_nausea_overlay_pipeline, create_hud_pipeline, create_hud_sprite_gpu,
        HudAdvancementBackgroundTexture, HudAdvancementHoverBoxSprite, HudAdvancementTabSprite,
        HudAdvancementWidgetFrameSprite, HudChatContext, HudChatLine, HudChatMessage,
        HudCrosshairAttackIndicator, HudDigitGlyph, HudFontGlyphBook, HudFontGlyphMap,
        HudHotbarMode, HudInventoryScreen, HudItemIcon, HudNineSliceSprite, HudObfuscatedGlyphPool,
        HudPlayerListPingSprite, HudSmithingSlotIcon, HudSpriteGpu, HudStatsScrollbarSprite,
        HudStatsSprite, HUD_HOTBAR_SLOTS,
    },
    item_entities::{create_item_entity_pipeline, ItemEntityAtlasGpu, ItemEntityBillboard},
    item_models::{
        create_display_text_background_pipeline, create_display_text_pipeline,
        create_display_text_see_through_pipeline, create_entity_shadow_pipeline,
        create_item_model_glint_pipeline, create_item_model_pipeline,
        create_item_model_translucent_pipeline, create_item_model_z_offset_forward_pipeline,
        create_resident_item_model_cutout_no_cull_pipeline,
        create_resident_item_model_cutout_z_offset_pipeline,
        create_resident_item_model_end_gateway_pipeline,
        create_resident_item_model_end_portal_pipeline, create_resident_item_model_glint_pipeline,
        create_resident_item_model_outline_pipeline, create_resident_item_model_pattern_pipeline,
        create_resident_item_model_pipeline, create_resident_item_model_solid_no_cutout_pipeline,
        create_resident_item_model_translucent_no_cull_pipeline,
        create_resident_item_model_translucent_pipeline,
        create_resident_item_model_z_offset_forward_pipeline, DisplayTextSurface,
        EntityNameTagSurface, FirstPersonMapBackgroundAtlasGpu, GlintTextureGpu,
        ItemFrameMapAtlasGpu, ItemFrameMapDecorationAtlasGpu, ItemFrameMapTextFontAtlasGpu,
        ItemModelMesh, ItemModelResidency, MapSurfaceResidency, SignTextOutlineContext,
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
    player_skin::{DynamicEntityTextureImage, DynamicPlayerSkinImage},
    post_effect_graph::{PostEffectGraphRuntime, PostEffectGraphRuntimeResize},
    screen_blur::{
        create_screen_blur_bind_group_layout, create_screen_blur_horizontal_pipeline,
        create_screen_blur_target, create_screen_blur_vertical_pipeline,
        sanitize_menu_background_blurriness, ScreenBlurTarget,
        VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
    },
    selection::{
        create_block_selection_gpu, create_chunk_border_always_on_top_pipeline,
        create_selection_outline_gpu, create_selection_pipeline,
        create_translucent_debug_line_pipeline, sanitize_debug_lines, BlockSelectionGpu,
        BlockSelectionRenderState, DebugLine, SelectionOutline, SelectionOutlineGpu,
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
    terrain_animation::TerrainAtlasAnimationGpu,
    transparency::{
        create_item_entity_target, create_main_target, create_particle_target,
        create_translucent_target, create_transparency_blit_bind_group_layout,
        create_transparency_blit_pipeline, create_transparency_combine_bind_group,
        create_transparency_combine_bind_group_layout, create_transparency_combine_pipeline,
        create_transparency_final_target, create_weather_target, ItemEntityTarget, MainTarget,
        ParticleTarget, TranslucentTarget, TransparencyCombineBindGroup, TransparencyFinalTarget,
        WeatherTarget,
    },
    vignette::{create_vignette_bind_group_layout, create_vignette_pipeline, VignetteTextureGpu},
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

/// Immutable projection of Vanilla's per-frame transparency routing decision.
///
/// `LevelRenderer` owns the actual target selection; native startup only
/// projects the option and panoramic-camera inputs into this narrow value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LevelFrameRoute {
    #[default]
    Regular,
    ImprovedTransparency,
}

impl LevelFrameRoute {
    pub fn for_options(improved_transparency: bool, panoramic: bool) -> Self {
        if improved_transparency && !panoramic {
            Self::ImprovedTransparency
        } else {
            Self::Regular
        }
    }

    pub(super) fn uses_improved_transparency(self) -> bool {
        self == Self::ImprovedTransparency
    }
}

#[cfg(feature = "lifecycle-soak")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleSoakSurfaceError {
    Lost,
    Outdated,
    Timeout,
}

#[cfg(feature = "lifecycle-soak")]
impl LifecycleSoakSurfaceError {
    fn into_surface_error(self) -> wgpu::SurfaceError {
        match self {
            Self::Lost => wgpu::SurfaceError::Lost,
            Self::Outdated => wgpu::SurfaceError::Outdated,
            Self::Timeout => wgpu::SurfaceError::Timeout,
        }
    }
}

impl RenderSurface {
    /// Acquires this frame's color target. `Ok(None)` means "skip this frame"
    /// — the swapchain was lost/outdated (reconfigured here, vanilla-style) or
    /// timed out — preserving the pre-harness `render()` surface semantics.
    pub(super) fn acquire_frame(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        counters: &mut RendererCounters,
        injected_fault: Option<wgpu::SurfaceError>,
    ) -> Result<Option<FrameTarget>> {
        let acquired = match injected_fault {
            Some(error) => Err(error),
            None => match self {
                Self::Window(surface) => surface.get_current_texture().map(FrameTarget::Surface),
                #[cfg(test)]
                Self::Offscreen(texture) => Ok(FrameTarget::Offscreen(texture.clone())),
            },
        };
        match acquired {
            Ok(frame) => Ok(Some(frame)),
            Err(wgpu::SurfaceError::Lost) => {
                counters.surface_lost = counters.surface_lost.saturating_add(1);
                counters.surface_reconfigurations =
                    counters.surface_reconfigurations.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                self.configure(device, config);
                Ok(None)
            }
            Err(wgpu::SurfaceError::Outdated) => {
                counters.surface_outdated = counters.surface_outdated.saturating_add(1);
                counters.surface_reconfigurations =
                    counters.surface_reconfigurations.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                self.configure(device, config);
                Ok(None)
            }
            Err(wgpu::SurfaceError::Timeout) => {
                counters.surface_timeouts = counters.surface_timeouts.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn configure(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        match self {
            Self::Window(surface) => surface.configure(device, config),
            #[cfg(test)]
            Self::Offscreen(texture) => {
                *texture = std::sync::Arc::new(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("bbb-offscreen-frame-target"),
                    size: wgpu::Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: config.format,
                    usage: config.usage,
                    view_formats: &[],
                }));
            }
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RendererFrameProfile {
    pub frame_interval: Duration,
    pub gpu_async: Duration,
    pub surface_acquire: Duration,
    pub render: Duration,
    pub present: Duration,
    pub swap_buffers: Duration,
}

/// Complete camera-uniform inputs extracted for one presented frame.
///
/// Vanilla composes the camera pose, hurt/view-bob/spinning transforms, FOV,
/// fog and game-time state before publishing the world and HUD projections.
/// Keeping the same boundary here lets the native runtime replace all inputs
/// before one GPU uniform refresh instead of exposing intermediate states to
/// several redundant queue writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RendererCameraFrame {
    pub lightmap_environment: LightmapEnvironment,
    pub fog_environment: FogEnvironment,
    pub sky_environment: SkyEnvironment,
    pub pose: Option<CameraPose>,
    pub hurt_effect: Option<CameraHurtEffect>,
    pub view_bob_effect: Option<CameraViewBobEffect>,
    pub spinning_effect: Option<CameraSpinningEffect>,
    pub fov_degrees: f32,
    pub hud_fov_degrees: f32,
    pub shader_game_time_ticks: f64,
}

impl Default for RendererCameraFrame {
    fn default() -> Self {
        Self {
            lightmap_environment: LightmapEnvironment::default(),
            fog_environment: FogEnvironment::default(),
            sky_environment: SkyEnvironment::default(),
            pose: None,
            hurt_effect: None,
            view_bob_effect: None,
            spinning_effect: None,
            fov_degrees: 70.0,
            hud_fov_degrees: 70.0,
            shader_game_time_ticks: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainChunkMeshOwner {
    pub level_generation: u64,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainSectionMeshOwner {
    pub chunk: TerrainChunkMeshOwner,
    pub section_y: i32,
}

/// Terrain residency keys are trusted packed coordinates, mirroring
/// Vanilla's primitive-long fastutil maps. Avoid SipHash on every visible
/// section while retaining collision-safe HashMap/HashSet equality semantics.
#[derive(Debug, Clone, Copy, Default)]
struct TerrainMeshOwnerBuildHasher;

#[derive(Debug, Clone, Copy)]
struct TerrainMeshOwnerHasher(u64);

impl BuildHasher for TerrainMeshOwnerBuildHasher {
    type Hasher = TerrainMeshOwnerHasher;

    fn build_hasher(&self) -> Self::Hasher {
        TerrainMeshOwnerHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for TerrainMeshOwnerHasher {
    fn finish(&self) -> u64 {
        // MurmurHash3's finalizer preserves the cheap word-at-a-time input path below while
        // spreading adjacent chunk/section coordinates across both hashbrown's bucket bits and
        // its seven-bit control fingerprint.
        let mut hash = self.0;
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        hash ^ (hash >> 33)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 ^= value;
        self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
    }

    fn write_i32(&mut self, value: i32) {
        self.write_u64(u64::from(value as u32));
    }
}

type TerrainSectionMap<V> = HashMap<TerrainSectionMeshOwner, V, TerrainMeshOwnerBuildHasher>;
type TerrainSectionSet = HashSet<TerrainSectionMeshOwner, TerrainMeshOwnerBuildHasher>;

/// Immutable native section-lifecycle projection consumed by the terrain GPU
/// owner in near-to-far traversal order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainSectionRenderState {
    pub owner: TerrainSectionMeshOwner,
    pub visibility: f32,
    /// Vanilla `SectionOcclusionGraph.addSectionsInFrustum`'s `isClose`
    /// projection, consumed by nearby-first translucent resort scheduling.
    pub nearby: bool,
}

/// Vanilla 26.1 terrain texture filtering modes currently owned by the native renderer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TerrainTextureFiltering {
    /// Vanilla's `TextureFilteringMethod.NONE` custom nearest-looking shader path.
    #[default]
    None,
    /// Vanilla's four-tap rotated-grid super-sampling shader path.
    Rgss,
    /// Vanilla's `TextureFilteringMethod.ANISOTROPIC`: the same nearest-looking
    /// shader path, with sampler anisotropy and matching atlas padding.
    Anisotropic,
}

/// Vanilla `Options.effectiveMaxAnisotropy` is `min(1 << maxAnisotropyBit,
/// device max)`; `LevelRenderer` passes it to the chunk-layer sampler only
/// under `TextureFilteringMethod.ANISOTROPIC`, else 1. wgpu has no queryable
/// device maximum, so the bit range 1..=3 (2x/4x/8x) is clamped to wgpu's
/// 16x sampler ceiling and the backend clamps to hardware support.
pub(crate) fn effective_terrain_max_anisotropy(
    filtering: TerrainTextureFiltering,
    max_anisotropy_bit: u32,
) -> u16 {
    if filtering != TerrainTextureFiltering::Anisotropic {
        return 1;
    }
    1_u32
        .checked_shl(max_anisotropy_bit)
        .unwrap_or(u32::from(u16::MAX))
        .min(16) as u16
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RendererResizePrepareStage {
    PostEffects,
    FrameTargets,
}

struct RendererResizePlan {
    size: PhysicalSize<u32>,
    post_effect_graphs: Option<PostEffectGraphRuntimeResize>,
    screen_blur_target: ScreenBlurTarget,
    camera_post_effect: CameraPostEffectResize,
    main_target: MainTarget,
    depth: DepthTarget,
    translucent_target: TranslucentTarget,
    item_entity_target: ItemEntityTarget,
    particle_target: ParticleTarget,
    weather_target: WeatherTarget,
    transparency_final_target: TransparencyFinalTarget,
    entity_outline_target: EntityOutlineTarget,
    cloud_target: CloudTarget,
    transparency_combine_bind_group: TransparencyCombineBindGroup,
}

pub struct Renderer {
    pub(super) surface: RenderSurface,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) config: wgpu::SurfaceConfiguration,
    /// The present modes the surface exposed at construction, so
    /// `set_vsync_enabled` can renegotiate `updateVsync` (Window.java:246)
    /// without recreating the adapter.
    pub(super) present_modes: Vec<wgpu::PresentMode>,
    /// `Options.enableVsync`: re-applied on fullscreen changes and forced off
    /// by `Window.updateVsync`. Winit exposes no synchronous fullscreen
    /// failure, so `Minecraft.onFullscreenError` has no trigger on this
    /// backend.
    pub(super) vsync_enabled: bool,
    /// The projected logical GUI surface (`Window.setGuiScale` ceiling
    /// division, `Minecraft.resizeGui`). HUD layout, GUI hit targets, toasts,
    /// the GUI item ortho camera and scissors consume this; `size`/`config`
    /// remain the physical framebuffer for the world camera, culling,
    /// counters and screenshots. A zero-size physical resize keeps the last
    /// usable GUI surface until a nonzero size republishes it.
    pub(super) gui_surface_size: PhysicalSize<u32>,
    /// Test-only counter of full camera-uniform refreshes (`update_camera`),
    /// covering the world, HUD-3D and GUI-item ortho buffers. The renderer
    /// test pins that `set_gui_surface_size` refreshes the dependent camera
    /// uniforms on change and no-ops when unchanged. Production keeps
    /// `update_camera(&self)`; the counter lives behind `#[cfg(test)]` so a
    /// test seam never mutates production state.
    #[cfg(test)]
    pub(super) camera_uniform_writes: std::cell::Cell<u64>,
    pub(super) size: PhysicalSize<u32>,
    pub(super) clear: ClearColor,
    pub(super) level_frame_route: LevelFrameRoute,
    pub(super) counters: RendererCounters,
    #[cfg(feature = "lifecycle-soak")]
    pub(super) lifecycle_soak_surface_faults: std::collections::VecDeque<wgpu::SurfaceError>,
    pub(super) started_at: Instant,
    pub(super) gpu_timing: Option<GpuFrameTiming>,
    pub(super) last_frame_profile: RendererFrameProfile,
    pub(super) last_frame_started_at: Instant,
    pub(super) shader_game_time_ticks: f64,
    /// Vanilla `optionsRenderState.glintSpeed` (`Options.glintSpeed`).
    pub(super) glint_speed: f64,
    /// Vanilla `optionsRenderState.glintStrength` (`Options.glintStrength`),
    /// the `GlobalSettingsUniform` glint alpha.
    pub(super) glint_alpha: f32,
    pub(super) current_post_effect_id: Option<String>,
    pub(super) camera_post_effect_active: bool,
    pub(super) camera_post_effect: CameraPostEffectGpu,
    pub(super) post_effect_graphs: Option<PostEffectGraphRuntime>,
    pub(super) menu_background_blurriness: f32,
    pub(super) hud_text_background_opacity: f32,
    pub(super) hud_background_for_chat_only: bool,
    pub(super) hide_gui: bool,
    pub(super) hud_screen_blur: bool,
    pub(super) screen_blur_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) screen_blur_horizontal_pipeline: wgpu::RenderPipeline,
    pub(super) screen_blur_vertical_pipeline: wgpu::RenderPipeline,
    pub(super) screen_blur_target: ScreenBlurTarget,
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
    pub(super) terrain_solid_pipeline: wgpu::RenderPipeline,
    pub(super) terrain_cutout_pipeline: wgpu::RenderPipeline,
    pub(super) terrain_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) core_shader_generation: Option<u64>,
    pub(super) terrain_texture_filtering: TerrainTextureFiltering,
    pub(super) terrain_max_anisotropy_bit: u32,
    pub(super) block_destroy_pipeline: wgpu::RenderPipeline,
    pub(super) debug_filled_box_pipeline: wgpu::RenderPipeline,
    pub(super) debug_text_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_textured_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_textured_cull_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_solid_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_dissolve_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_solid_z_offset_forward_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_cutout_z_offset_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_armor_cutout_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_armor_decal_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_banner_pattern_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_translucent_cull_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_armor_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_translucent_emissive_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_eyes_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_dragon_rays_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_dragon_rays_depth_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_leash_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_end_portal_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_end_gateway_pipeline: wgpu::RenderPipeline,
    pub(super) hud_win_portal_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_water_mask_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_outline_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_outline_cull_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_scroll_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_scroll_additive_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_guardian_beam_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_end_crystal_beam_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_beacon_beam_opaque_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_beacon_beam_translucent_pipeline: wgpu::RenderPipeline,
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
    pub(super) frame_debug_filled_box_vertices: FrameDataBuffer,
    pub(super) frame_debug_filled_box_indices: FrameDataBuffer,
    pub(super) frame_debug_text_vertices: FrameDataBuffer,
    pub(super) frame_debug_text_indices: FrameDataBuffer,
    pub(super) frame_debug_always_on_top_text_vertices: FrameDataBuffer,
    pub(super) frame_debug_always_on_top_text_indices: FrameDataBuffer,
    pub(super) frame_debug_line_opaque_vertices: FrameDataBuffer,
    pub(super) frame_debug_line_translucent_vertices: FrameDataBuffer,
    pub(super) frame_entity_model_line_vertices: FrameDataBuffer,
    pub(super) frame_block_entity_crumbling_vertices: FrameDataBuffer,
    pub(super) frame_block_entity_crumbling_indices: FrameDataBuffer,
    pub(super) frame_item_entity_vertices: FrameDataBuffer,
    pub(super) frame_hud_vertices: FrameDataBuffer,
    pub(super) weather_pipeline: wgpu::RenderPipeline,
    pub(super) lightning_pipeline: wgpu::RenderPipeline,
    pub(super) world_border_pipeline: wgpu::RenderPipeline,
    pub(super) item_entity_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_pipeline: wgpu::RenderPipeline,
    pub(super) entity_shadow_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_z_offset_forward_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) item_model_glint_pipeline: wgpu::RenderPipeline,
    pub(super) display_text_background_pipeline: wgpu::RenderPipeline,
    pub(super) display_text_pipeline: wgpu::RenderPipeline,
    pub(super) display_text_see_through_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_solid_no_cutout_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_z_offset_forward_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_cutout_no_cull_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_cutout_z_offset_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_pattern_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_translucent_no_cull_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_glint_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_end_portal_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_end_gateway_pipeline: wgpu::RenderPipeline,
    pub(super) resident_item_model_outline_pipeline: wgpu::RenderPipeline,
    pub(super) hud_item_glint_pipeline: wgpu::RenderPipeline,
    pub(super) selection_pipeline: wgpu::RenderPipeline,
    pub(super) translucent_debug_line_pipeline: wgpu::RenderPipeline,
    pub(super) chunk_border_always_on_top_pipeline: wgpu::RenderPipeline,
    pub(super) lightmap_pipeline: wgpu::RenderPipeline,
    pub(super) lightmap: LightmapGpu,
    pub(super) lightmap_dirty: bool,
    pub(super) lightmap_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) lightmap_hud_bind_group: wgpu::BindGroup,
    pub(super) _lightmap_hud_sampler: wgpu::Sampler,
    pub(super) lightmap_sample_bind_group_layout: wgpu::BindGroupLayout,
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
    pub(super) cloud_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) cloud_bind_group: wgpu::BindGroup,
    pub(super) cloud_uniform_buffer: wgpu::Buffer,
    pub(super) hud_pipeline: wgpu::RenderPipeline,
    pub(super) hud_nausea_overlay_pipeline: wgpu::RenderPipeline,
    pub(super) hud_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) vignette_pipeline: wgpu::RenderPipeline,
    pub(super) vignette_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) screen_effect_pipeline: wgpu::RenderPipeline,
    pub(super) screen_effect_quads: Vec<crate::screen_effects::ScreenEffectQuad>,
    pub(super) screen_effect_underwater: Option<crate::screen_effects::ScreenEffectUnderwaterGpu>,
    pub(super) hud_white_pixel: HudSpriteGpu,
    pub(super) hud_black_pixel: HudSpriteGpu,
    pub(super) terrain_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) terrain_section_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) camera_buffer: wgpu::Buffer,
    pub(super) hud3d_camera_buffer: wgpu::Buffer,
    pub(super) gui_item_camera_buffer: wgpu::Buffer,
    pub(super) terrain_atlas: TerrainAtlasGpu,
    pub(super) terrain_atlas_animation: Option<TerrainAtlasAnimationGpu>,
    pub(super) terrain_bind_group: wgpu::BindGroup,
    pub(super) hud3d_terrain_bind_group: wgpu::BindGroup,
    pub(super) hud_terrain_atlas_bind_group: wgpu::BindGroup,
    pub(super) gui_item_bind_group: wgpu::BindGroup,
    pub(super) hud_hotbar_block_item_models: Vec<Option<crate::item_models::HudBlockItemModel>>,
    pub(super) hud_offhand_slot: Option<crate::hud::HudOffhandSlot>,
    pub(super) hud_offhand_block_item_model: Option<crate::item_models::HudBlockItemModel>,
    pub(super) hud_hotbar_attack_indicator: crate::hud::HudHotbarAttackIndicator,
    pub(super) terrain_opaque: Vec<ResidentTerrainMesh>,
    pub(super) terrain_cutout: Vec<ResidentTerrainMesh>,
    pub(super) terrain_translucent: Vec<ResidentTerrainMesh>,
    terrain_section_revisions: TerrainSectionMap<u64>,
    terrain_section_visibility: TerrainSectionMap<f32>,
    terrain_next_section_visibility: TerrainSectionMap<f32>,
    terrain_section_visibility_has_fading: bool,
    #[cfg(test)]
    terrain_section_uniform_scan_count: usize,
    terrain_visible_section_order: Vec<TerrainSectionMeshOwner>,
    terrain_next_visible_section_order: Vec<TerrainSectionMeshOwner>,
    terrain_nearby_visible_section_order: Vec<TerrainSectionMeshOwner>,
    terrain_next_nearby_visible_section_order: Vec<TerrainSectionMeshOwner>,
    terrain_translucent_indices: TerrainSectionMap<usize>,
    terrain_section_states_revision: Option<u64>,
    // Cross-section back-to-front draw order (far→near) of indices into
    // `terrain_translucent`. Vanilla preserves the LevelRenderer visible-section
    // traversal and reverses that draw list for TRANSLUCENT
    // (ChunkSectionsToRender.java:55-56, MC 26.1). Keep both the membership set
    // and that authoritative traversal order; see
    // `rebuild_translucent_section_order`.
    pub(super) terrain_translucent_order: Vec<usize>,
    last_translucent_sort_block_position: Option<[i32; 3]>,
    translucency_resort_iteration_index: usize,
    translucency_resort_drain_index: usize,
    pub(super) terrain_source_sections: usize,
    pub(super) terrain_bounds: Option<TerrainBounds>,
    pub(super) entity_model_bounds: Option<TerrainBounds>,
    pub(super) camera_pose: Option<CameraPose>,
    pub(super) camera_hurt_effect: Option<CameraHurtEffect>,
    pub(super) camera_view_bob_effect: Option<CameraViewBobEffect>,
    pub(super) camera_spinning_effect: Option<CameraSpinningEffect>,
    pub(super) camera_fov_degrees: f32,
    pub(super) camera_hud_fov_degrees: f32,
    pub(super) lightmap_environment: LightmapEnvironment,
    pub(super) fog_environment: FogEnvironment,
    pub(super) sky_environment: SkyEnvironment,
    pub(super) cloud_environment: CloudEnvironment,
    pub(super) cloud_frame: CloudFrame,
    pub(super) cloud_shape: CloudShape,
    /// Vanilla `CloudStatus.OFF` (`renderClouds: "false"`): `LevelRenderer`
    /// skips the cloud pass entirely.
    pub(super) clouds_enabled: bool,
    pub(super) sky_disc: Option<SkyDiscGpu>,
    pub(super) end_sky_mesh: EndSkyGpu,
    pub(super) end_sky_texture: Option<EndSkyTextureGpu>,
    pub(super) sky_celestials: Option<CelestialGpu>,
    pub(super) sky_stars: Option<StarGpu>,
    pub(super) celestial_atlas: Option<CelestialAtlasGpu>,
    pub(super) cloud_texture: Option<CloudTextureData>,
    pub(super) clouds: Option<CloudGpu>,
    pub(super) block_destroy_overlays: Option<BlockDestroyOverlaysGpu>,
    pub(super) block_entity_crumbling_index_count: u32,
    pub(super) entity_model_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_water_mask_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_solid_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_textured_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_textured_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_dissolve_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_solid_z_offset_forward_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_cutout_z_offset_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_armor_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_armor_trim_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_armor_trim_decal_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_armor_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_translucent_emissive_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_item_entity_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_item_entity_translucent_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_opaque_beam_draws: Vec<EntityModelScrollDrawRange>,
    pub(super) entity_model_sorted_main_translucent_draws: Vec<EntityModelTranslucentDrawRange>,
    pub(super) entity_model_sorted_translucent_draws: Vec<EntityModelTexturedDrawRange>,
    pub(super) entity_model_sorted_item_entity_draws: Vec<EntityModelTexturedDrawRange>,
    pub(super) entity_model_eyes_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_dragon_rays_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_dragon_rays_depth_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_leash_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_end_portal_mesh: Option<EntityModelPortalMeshGpu>,
    pub(super) entity_model_end_gateway_mesh: Option<EntityModelPortalMeshGpu>,
    pub(super) hud_win_portal_mesh: Option<EntityModelPortalMeshGpu>,
    pub(super) entity_model_outline_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_outline_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_entity_glint_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_armor_entity_glint_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_dynamic_player_skin_solid_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_cutout_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_solid_z_offset_forward_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_cutout_z_offset_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_item_entity_translucent_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_skin_item_entity_translucent_cull_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_solid_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_cutout_cull_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_solid_z_offset_forward_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_cutout_z_offset_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_armor_cutout_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_armor_trim_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_armor_trim_decal_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_translucent_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_item_entity_translucent_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_dynamic_player_texture_item_entity_translucent_cull_mesh:
        Option<EntityModelTexturedMeshGpu>,
    pub(super) first_person_player_arm_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) first_person_dynamic_player_arm_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_scroll_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_scroll_additive_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_guardian_beam_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_end_crystal_beam_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_beacon_beam_opaque_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_beacon_beam_translucent_mesh: Option<EntityModelScrollMeshGpu>,
    pub(super) entity_model_texture_atlas: Option<EntityModelTextureAtlasGpu>,
    pub(super) entity_dynamic_player_skin_atlas: Option<EntityDynamicPlayerSkinAtlasGpu>,
    pub(super) entity_dynamic_player_skin_images: Vec<DynamicPlayerSkinImage>,
    pub(super) entity_dynamic_player_texture_atlas: Option<EntityDynamicPlayerTextureAtlasGpu>,
    pub(super) entity_dynamic_player_texture_images: Vec<DynamicEntityTextureImage>,
    pub(super) dynamic_entity_texture_generation: Option<(u64, u64, bool)>,
    pub(super) entity_model_instances: Vec<crate::EntityModelInstance>,
    pub(super) entity_flame_surfaces: Vec<EntityFlameSurface>,
    pub(super) entity_shadow_surfaces: Vec<EntityShadowSurface>,
    pub(super) entity_shadow_texture: Option<EntityShadowTextureGpu>,
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
    pub(super) item_glint_texture: Option<GlintTextureGpu>,
    pub(super) armor_glint_texture: Option<GlintTextureGpu>,
    pub(super) item_entity_billboards: Vec<ItemEntityBillboard>,
    pub(super) item_model_residency: ItemModelResidency,
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
    #[cfg(test)]
    pub(super) item_frame_map_surfaces: Vec<crate::item_models::ItemFrameMapSurface>,
    #[cfg(test)]
    pub(super) first_person_map_surfaces: Vec<crate::item_models::ItemFrameMapSurface>,
    #[cfg(test)]
    pub(super) first_person_map_background_surfaces:
        Vec<crate::item_models::FirstPersonMapBackgroundSurface>,
    pub(super) first_person_map_background_atlas: Option<FirstPersonMapBackgroundAtlasGpu>,
    pub(super) item_frame_map_atlas: Option<ItemFrameMapAtlasGpu>,
    #[cfg(test)]
    pub(super) item_frame_map_decoration_surfaces:
        Vec<crate::item_models::ItemFrameMapDecorationSurface>,
    #[cfg(test)]
    pub(super) first_person_map_decoration_surfaces:
        Vec<crate::item_models::ItemFrameMapDecorationSurface>,
    pub(super) item_frame_map_decoration_atlas: Option<ItemFrameMapDecorationAtlasGpu>,
    pub(super) sign_text_surfaces: Vec<SignTextSurface>,
    pub(super) visible_sign_text_positions: Vec<[i32; 3]>,
    pub(super) sign_text_outline_context: SignTextOutlineContext,
    pub(super) display_text_surfaces: Vec<DisplayTextSurface>,
    pub(super) entity_name_tag_surfaces: Vec<EntityNameTagSurface>,
    pub(super) item_frame_map_text_font_atlas: Option<ItemFrameMapTextFontAtlasGpu>,
    pub(super) map_surface_residency: MapSurfaceResidency,
    pub(super) block_selection: Option<BlockSelectionGpu>,
    pub(super) chunk_border_outline: Option<SelectionOutlineGpu>,
    pub(super) entity_scene_outline: Option<SelectionOutlineGpu>,
    pub(super) debug_filled_boxes: Vec<DebugFilledBox>,
    pub(super) debug_filled_rects: Vec<DebugFilledRect>,
    pub(super) debug_text_billboards: Vec<DebugTextBillboard>,
    pub(super) debug_lines: Vec<DebugLine>,
    /// Revision-keyed GameTest values follow the per-frame debug renderer
    /// values in Vanilla's gizmo insertion order.
    pub(super) game_test_highlight_boxes: Vec<DebugFilledBox>,
    pub(super) game_test_highlight_text_billboards: Vec<DebugTextBillboard>,
    pub(super) hud_spyglass_scope_texture: Option<HudSpriteGpu>,
    pub(super) hud_camera_overlay_textures: HashMap<String, HudSpriteGpu>,
    pub(super) hud_camera_overlays: Vec<crate::hud::HudCameraOverlay>,
    pub(super) hud_portal_overlay: Option<crate::hud::HudPortalOverlay>,
    pub(super) hud_confusion_overlay: Option<crate::hud::HudConfusionOverlay>,
    pub(super) hud_nausea_overlay_texture: Option<HudSpriteGpu>,
    pub(super) hud_crosshair: Option<HudSpriteGpu>,
    pub(super) hud_crosshair_visible: bool,
    pub(super) hud_crosshair_attack_indicator_full: Option<HudSpriteGpu>,
    pub(super) hud_crosshair_attack_indicator_background: Option<HudSpriteGpu>,
    pub(super) hud_crosshair_attack_indicator_progress: Option<HudSpriteGpu>,
    pub(super) hud_crosshair_attack_indicator: HudCrosshairAttackIndicator,
    pub(super) hud_hotbar: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_selection: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_offhand_left: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_offhand_right: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_attack_indicator_background: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_attack_indicator_progress: Option<HudSpriteGpu>,
    pub(super) hud_spectator_close: Option<HudSpriteGpu>,
    pub(super) hud_spectator_scroll_left: Option<HudSpriteGpu>,
    pub(super) hud_spectator_scroll_right: Option<HudSpriteGpu>,
    pub(super) hud_spectator_teleport_to_player: Option<HudSpriteGpu>,
    pub(super) hud_spectator_teleport_to_team: Option<HudSpriteGpu>,
    pub(super) hud_default_player_faces:
        HashMap<EntityDefaultPlayerSkin, crate::hud::HudPlayerFaceGpu>,
    pub(super) hud_dynamic_player_faces: HashMap<u64, crate::hud::HudPlayerFaceGpu>,
    pub(super) hud_player_list_ping_sprites:
        [Option<HudSpriteGpu>; HudPlayerListPingSprite::ALL.len()],
    pub(super) hud_social_interactions_background: Option<HudNineSliceSprite>,
    pub(super) hud_social_interactions_search: Option<HudSpriteGpu>,
    pub(super) hud_social_interactions_mute: Option<HudSpriteGpu>,
    pub(super) hud_social_interactions_mute_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_social_interactions_unmute: Option<HudSpriteGpu>,
    pub(super) hud_social_interactions_unmute_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_item_atlas: Option<HudSpriteGpu>,
    pub(super) hud_digit_atlas: Option<HudSpriteGpu>,
    pub(super) hud_digit_glyphs: [HudDigitGlyph; 10],
    pub(super) hud_font_atlas: Option<HudSpriteGpu>,
    pub(super) hud_font_glyphs: HudFontGlyphMap,
    pub(super) hud_font_book: HudFontGlyphBook,
    /// Advance-grouped glyph pool for obfuscated (`§k`) text, rebuilt once when
    /// `hud_font_glyphs` is uploaded so per-frame draws never rescan the table.
    pub(super) hud_obfuscated_glyph_pool: HudObfuscatedGlyphPool,
    pub(super) hud_chat_modified_icon: Option<HudSpriteGpu>,
    pub(super) hud_draft_report_icon: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_item_icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS],
    pub(super) hud_inworld_menu_background: Option<HudSpriteGpu>,
    pub(super) hud_inventory_background: Option<HudSpriteGpu>,
    pub(super) hud_creative_items_background: Option<HudSpriteGpu>,
    pub(super) hud_creative_item_search_background: Option<HudSpriteGpu>,
    pub(super) hud_creative_inventory_background: Option<HudSpriteGpu>,
    pub(super) hud_creative_scroller: Option<HudSpriteGpu>,
    pub(super) hud_creative_scroller_disabled: Option<HudSpriteGpu>,
    pub(super) hud_creative_top_unselected_tabs: [Option<HudSpriteGpu>; 7],
    pub(super) hud_creative_top_selected_tabs: [Option<HudSpriteGpu>; 7],
    pub(super) hud_creative_bottom_unselected_tabs: [Option<HudSpriteGpu>; 7],
    pub(super) hud_creative_bottom_selected_tabs: [Option<HudSpriteGpu>; 7],
    pub(super) hud_tooltip_background: Option<HudNineSliceSprite>,
    pub(super) hud_tooltip_frame: Option<HudNineSliceSprite>,
    pub(super) hud_tooltip_styles: HashMap<String, crate::hud::HudTooltipStyleSprites>,
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
    pub(super) hud_widget_preedit: Option<HudNineSliceSprite>,
    pub(super) hud_widget_button: Option<HudSpriteGpu>,
    pub(super) hud_widget_button_disabled: Option<HudSpriteGpu>,
    pub(super) hud_widget_button_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_dialog_warning_button: Option<HudSpriteGpu>,
    pub(super) hud_dialog_warning_button_disabled: Option<HudSpriteGpu>,
    pub(super) hud_dialog_warning_button_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_widget_checkbox: Option<HudSpriteGpu>,
    pub(super) hud_widget_checkbox_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_widget_checkbox_selected: Option<HudSpriteGpu>,
    pub(super) hud_widget_checkbox_selected_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_widget_slider: Option<HudSpriteGpu>,
    pub(super) hud_widget_slider_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_widget_slider_handle: Option<HudSpriteGpu>,
    pub(super) hud_widget_slider_handle_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_stats_sprites: [Option<HudSpriteGpu>; HudStatsSprite::COUNT],
    pub(super) hud_stats_scrollbar_sprites:
        [Option<HudNineSliceSprite>; HudStatsScrollbarSprite::COUNT],
    pub(super) hud_system_toast_background: Option<HudNineSliceSprite>,
    pub(super) hud_system_toast: Option<crate::hud::HudSystemToast>,
    pub(super) hud_advancement_toast_background: Option<HudSpriteGpu>,
    pub(super) hud_advancement_toasts: Vec<crate::hud::HudAdvancementToast>,
    pub(super) hud_recipe_toast_background: Option<HudSpriteGpu>,
    pub(super) hud_recipe_toasts: Vec<crate::hud::HudRecipeToast>,
    pub(super) hud_tutorial_toast_background: Option<HudNineSliceSprite>,
    pub(super) hud_tutorial_toast_icons:
        [Option<HudSpriteGpu>; crate::hud::HudTutorialToastIcon::COUNT],
    pub(super) hud_tutorial_toasts: Vec<crate::hud::HudTutorialToast>,
    pub(super) hud_debug_game_mode_switcher_background: Option<HudSpriteGpu>,
    pub(super) hud_debug_game_mode_switcher_slot: Option<HudSpriteGpu>,
    pub(super) hud_debug_game_mode_switcher_selection: Option<HudSpriteGpu>,
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
    pub(super) hud_mob_effect_sprites: BTreeMap<i32, HudSpriteGpu>,
    pub(super) hud_effect_background: Option<HudSpriteGpu>,
    pub(super) hud_effect_background_ambient: Option<HudSpriteGpu>,
    pub(super) hud_inventory_effect_background: Option<HudNineSliceSprite>,
    pub(super) hud_inventory_effect_background_ambient: Option<HudNineSliceSprite>,
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
    pub(super) hud_smithing_slot_icons: [Option<HudSpriteGpu>; HudSmithingSlotIcon::COUNT],
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
    pub(super) hud_page_backward_highlighted: Option<HudSpriteGpu>,
    pub(super) hud_page_forward: Option<HudSpriteGpu>,
    pub(super) hud_page_forward_highlighted: Option<HudSpriteGpu>,
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
    pub(super) hud_command_block_editor_screen: Option<crate::hud::HudCommandBlockEditorScreen>,
    pub(super) hud_structure_block_editor_screen: Option<crate::hud::HudStructureBlockEditorScreen>,
    pub(super) hud_jigsaw_block_editor_screen: Option<crate::hud::HudJigsawBlockEditorScreen>,
    pub(super) hud_test_block_editor_screen: Option<crate::hud::HudTestBlockEditorScreen>,
    pub(super) hud_test_instance_block_editor_screen:
        Option<crate::hud::HudTestInstanceBlockEditorScreen>,
    pub(super) hud_pause_screen: Option<crate::hud::HudPauseScreen>,
    pub(super) hud_social_interactions_screen: Option<crate::hud::HudSocialInteractionsScreen>,
    pub(super) hud_player_reporting_screen: Option<crate::hud::HudPlayerReportingScreen>,
    pub(super) hud_stats_screen: Option<crate::hud::HudStatsScreen>,
    pub(super) hud_game_rules_screen: Option<crate::hud::HudGameRulesScreen>,
    pub(super) hud_death_screen: Option<crate::hud::HudDeathScreen>,
    pub(super) hud_disconnected_screen: Option<crate::hud::HudDisconnectedScreen>,
    pub(super) hud_level_loading_screen: Option<crate::hud::HudLevelLoadingScreen>,
    pub(super) hud_game_event_screen: Option<crate::hud::HudGameEventScreen>,
    pub(super) hud_book_screen: Option<crate::hud::HudBookScreen>,
    pub(super) hud_debug_options_screen: Option<crate::hud::HudDebugOptionsScreen>,
    pub(super) hud_sound_subtitles: crate::hud::HudSoundSubtitles,
    pub(super) hud_panorama: Option<crate::panorama::PanoramaGpu>,
    pub(super) hud_menu_background: Option<HudSpriteGpu>,
    /// One persistent PIP target per sanitized `hud_inventory_screen` entity preview, indexed by
    /// preview order (vanilla `PictureInPictureRenderer` private color+depth textures).
    pub(super) hud_entity_preview_pip_targets: Vec<HudEntityPreviewPipTarget>,
    pub(super) hud_experience_background: Option<HudSpriteGpu>,
    pub(super) hud_experience_progress: Option<HudSpriteGpu>,
    pub(super) hud_jump_bar_background: Option<HudSpriteGpu>,
    pub(super) hud_jump_bar_progress: Option<HudSpriteGpu>,
    pub(super) hud_jump_bar_cooldown: Option<HudSpriteGpu>,
    pub(super) hud_locator_background: Option<HudSpriteGpu>,
    pub(super) hud_locator_arrow_up: Option<HudSpriteGpu>,
    pub(super) hud_locator_arrow_down: Option<HudSpriteGpu>,
    pub(super) hud_locator_marker_sprites: BTreeMap<String, HudSpriteGpu>,
    pub(super) hud_locator_styles: BTreeMap<String, crate::hud::HudLocatorStyle>,
    /// Player heart sprites keyed by `[HudHeartKind as usize][variant]`, where
    /// `variant = (blinking as usize) * 4 + (hardcore as usize) * 2
    /// + (half as usize)`. `Container`'s half slots mirror its full slot
    /// (vanilla routes both to the container sprite).
    pub(super) hud_heart_sprites: [[Option<HudSpriteGpu>; 8]; crate::hud::HudHeartKind::ALL.len()],
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
    pub(super) hud_code_of_conduct_screen: Option<crate::hud::HudCodeOfConductScreen>,
    pub(super) hud_player_health: Option<crate::hud::HudPlayerHealth>,
    pub(super) hud_sleep_overlay: Option<crate::hud::HudSleepOverlay>,
    pub(super) hud_food: Option<i32>,
    pub(super) hud_food_effect: crate::hud::HudFoodEffect,
    pub(super) hud_mob_effects: Vec<crate::hud::HudMobEffect>,
    pub(super) hud_inventory_shows_mob_effects: bool,
    pub(super) hud_armor: Option<i32>,
    pub(super) hud_air: Option<crate::hud::HudAirSupply>,
    pub(super) hud_vehicle_health: Option<crate::hud::HudVehicleHealth>,
    pub(super) hud_vignette: Option<crate::vignette::HudVignette>,
    pub(super) hud_spyglass_scope: Option<crate::hud::HudSpyglassScope>,
    pub(super) hud_vignette_texture: Option<VignetteTextureGpu>,
    pub(super) hud_win_logo: Option<HudSpriteGpu>,
    pub(super) hud_win_easter_egg_logo: Option<HudSpriteGpu>,
    pub(super) hud_win_edition: Option<HudSpriteGpu>,
    pub(super) hud_win_credits_vignette: Option<VignetteTextureGpu>,
    pub(super) hud_demo_popup_background: Option<crate::hud::HudNineSliceSprite>,
    pub(super) hud_contextual_bar: crate::hud::HudContextualBar,
    pub(super) hud_experience_level: Option<i32>,
    pub(super) hud_hotbar_mode: HudHotbarMode,
    pub(super) hud_selected_slot: u8,
    pub(super) hud_action_bar_text: Option<crate::hud::HudActionBarText>,
    pub(super) hud_selected_item_name: Option<crate::hud::HudSelectedItemName>,
    pub(super) hud_title_text: Option<crate::hud::HudTitleText>,
    pub(super) hud_scoreboard_sidebar: Option<crate::hud::HudScoreboardSidebar>,
    pub(super) hud_player_list: Option<crate::hud::HudPlayerList>,
    pub(super) hud_player_list_runtime: crate::hud::HudPlayerListRuntime,
    pub(super) hud_chat_messages: Vec<HudChatMessage>,
    pub(super) hud_chat_lines: Vec<HudChatLine>,
    pub(super) hud_chat_context: HudChatContext,
    pub(super) hud_chat_restrictions_screen: Option<crate::hud::HudChatRestrictionsScreen>,
    pub(super) hud_command_suggestions: Option<crate::hud::HudCommandSuggestions>,
    pub(super) hud_debug_overlay: Option<crate::hud::HudDebugOverlay>,
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
    pub(super) resource_reload_checkpoint:
        Option<crate::resource_reload::RendererResourceReloadCheckpoint>,
}

pub(super) struct ResidentTerrainMesh {
    owner: Option<TerrainSectionMeshOwner>,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) vertex_count: usize,
    pub(super) index_count: usize,
    pub(super) opaque_faces: usize,
    pub(super) cutout_faces: usize,
    pub(super) translucent_faces: usize,
    pub(super) culled_faces: usize,
    pub(super) resident_bytes: u64,
    section_visibility: f32,
    _section_uniform_buffer: wgpu::Buffer,
    pub(super) section_bind_group: wgpu::BindGroup,
    // Axis-aligned bounds of this section's translucent geometry; its center is
    // the sort key for the cross-section back-to-front draw order. `None` only
    // for degenerate (empty-vertex) meshes, which are filtered before residency.
    bounds: Option<TerrainBounds>,
    translucent_sort: Option<TerrainTranslucentSortState>,
}

fn retain_resident_terrain_sections(
    meshes: &mut Vec<ResidentTerrainMesh>,
    desired_sections: &TerrainSectionSet,
) -> usize {
    let before = meshes.len();
    meshes.retain(|mesh| {
        mesh.owner
            .is_some_and(|owner| desired_sections.contains(&owner))
    });
    before - meshes.len()
}

fn remove_resident_terrain_section(
    meshes: &mut Vec<ResidentTerrainMesh>,
    section: TerrainSectionMeshOwner,
) -> usize {
    let before = meshes.len();
    meshes.retain(|mesh| mesh.owner != Some(section));
    before - meshes.len()
}

#[derive(Debug, Clone)]
struct TerrainTranslucentSortState {
    centroids: Vec<[f32; 3]>,
    local_indices: Vec<[u32; 6]>,
    last_point_of_view: Option<[i8; 3]>,
    pending_camera: Option<([f32; 3], [i8; 3])>,
}

impl TerrainTranslucentSortState {
    fn from_vertices(
        vertices: &[terrain::TerrainVertex],
        indices: &[u32],
        last_point_of_view: Option<[i8; 3]>,
    ) -> Option<Self> {
        if vertices.len() % 4 != 0 || indices.len() != vertices.len() / 4 * 6 {
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
            .collect::<Vec<_>>();
        let mut local_indices = vec![None; centroids.len()];
        for quad in indices.chunks_exact(6) {
            let quad_index = (quad[0] / 4) as usize;
            let base = (quad_index * 4) as u32;
            if quad_index >= local_indices.len()
                || quad
                    .iter()
                    .any(|index| !(*index >= base && *index < base + 4))
                || local_indices[quad_index].is_some()
            {
                return None;
            }
            local_indices[quad_index] = Some(std::array::from_fn(|index| quad[index] - base));
        }
        let local_indices = local_indices.into_iter().collect::<Option<Vec<_>>>()?;
        Some(Self {
            centroids,
            local_indices,
            last_point_of_view,
            pending_camera: None,
        })
    }

    fn schedule_for_camera_if_needed(
        &mut self,
        camera_position: [f32; 3],
        point_of_view: [i8; 3],
        block_position_changed: bool,
        nearby: bool,
    ) {
        if self.pending_camera.is_some() {
            // Vanilla's queued task reads the dispatcher's latest camera position
            // when a worker eventually executes it. Keep the same coalescing
            // behavior while this renderer-owned bounded queue is pending.
            self.pending_camera = Some((camera_position, point_of_view));
            return;
        }
        let point_of_view_changed = self.last_point_of_view != Some(point_of_view);
        let axis_aligned = point_of_view.into_iter().any(|coordinate| coordinate == 0);
        if !point_of_view_changed && !(block_position_changed && (axis_aligned || nearby)) {
            return;
        }
        self.pending_camera = Some((camera_position, point_of_view));
    }

    fn take_scheduled_indices(&mut self) -> Option<Vec<u32>> {
        let (camera_position, point_of_view) = self.pending_camera.take()?;
        self.last_point_of_view = Some(point_of_view);
        Some(self.sorted_indices(camera_position))
    }

    #[cfg(test)]
    fn indices_for_camera_if_needed(
        &mut self,
        camera_position: [f32; 3],
        point_of_view: [i8; 3],
        block_position_changed: bool,
        nearby: bool,
    ) -> Option<Vec<u32>> {
        self.schedule_for_camera_if_needed(
            camera_position,
            point_of_view,
            block_position_changed,
            nearby,
        );
        self.take_scheduled_indices()
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
            indices.extend(
                self.local_indices[quad_index]
                    .into_iter()
                    .map(|index| base + index),
            );
        }
        indices
    }
}

// Vanilla performs translucent index sorting in SectionRenderDispatcher workers.
// BBB currently owns the cached CPU geometry in the renderer, so drain only a
// small number synchronously per camera update instead of sorting every nearby
// section on the render thread at a block boundary. Pending requests coalesce to
// the newest camera above, matching the worker task's late camera read.
const MAX_TRANSLUCENT_RESORTS_PER_CAMERA_UPDATE: usize = 1;

fn camera_block_position(camera_position: [f32; 3]) -> [i32; 3] {
    camera_position.map(|coordinate| coordinate.floor() as i32)
}

fn camera_section_position(camera_position: [f32; 3]) -> [i32; 3] {
    camera_block_position(camera_position).map(|coordinate| coordinate.div_euclid(16))
}

fn translucency_point_of_view(
    camera_position: [f32; 3],
    owner: TerrainSectionMeshOwner,
) -> [i8; 3] {
    let camera_section = camera_section_position(camera_position);
    [
        (camera_section[0] - owner.chunk.chunk_x).clamp(-1, 1) as i8,
        (camera_section[1] - owner.section_y).clamp(-1, 1) as i8,
        (camera_section[2] - owner.chunk.chunk_z).clamp(-1, 1) as i8,
    ]
}

fn translucent_section_is_nearby(
    camera_position: [f32; 3],
    owner: TerrainSectionMeshOwner,
) -> bool {
    let camera_section = camera_section_position(camera_position);
    (i64::from(camera_section[0]) - i64::from(owner.chunk.chunk_x)).abs() <= 2
        && (i64::from(camera_section[1]) - i64::from(owner.section_y)).abs() <= 2
        && (i64::from(camera_section[2]) - i64::from(owner.chunk.chunk_z)).abs() <= 2
}

fn schedule_translucent_section(
    meshes: &mut [ResidentTerrainMesh],
    resident_indices: &TerrainSectionMap<usize>,
    owner: TerrainSectionMeshOwner,
    camera_position: [f32; 3],
    block_position_changed: bool,
    nearby: bool,
) {
    let Some(&index) = resident_indices.get(&owner) else {
        return;
    };
    let Some(mesh) = meshes.get_mut(index) else {
        return;
    };
    debug_assert_eq!(mesh.owner, Some(owner));
    let Some(sort) = &mut mesh.translucent_sort else {
        return;
    };
    sort.schedule_for_camera_if_needed(
        camera_position,
        translucency_point_of_view(camera_position, owner),
        block_position_changed,
        nearby,
    );
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

/// Map Vanilla's near→far visible-section traversal to resident translucent
/// mesh indices, then reverse it exactly as `ChunkSectionsToRender.renderGroup`
/// does for the TRANSLUCENT layer. Meshes outside the visible traversal are appended in
/// storage order so the result remains a complete permutation; the draw pass
/// still rejects them through `terrain_mesh_is_visible`.
fn translucent_section_draw_order_from_visible(
    owners: &[Option<TerrainSectionMeshOwner>],
    visible_near_to_far: &[TerrainSectionMeshOwner],
) -> Vec<usize> {
    let owner_indices: BTreeMap<_, _> = owners
        .iter()
        .enumerate()
        .filter_map(|(index, owner)| owner.map(|owner| (owner, index)))
        .collect();
    let mut included = vec![false; owners.len()];
    let mut order = Vec::with_capacity(owners.len());
    for owner in visible_near_to_far.iter().rev() {
        let Some(&index) = owner_indices.get(owner) else {
            continue;
        };
        if !included[index] {
            included[index] = true;
            order.push(index);
        }
    }
    order.extend((0..owners.len()).filter(|&index| !included[index]));
    order
}

fn choose_present_mode(
    present_modes: &[wgpu::PresentMode],
    vsync_enabled: bool,
) -> Result<wgpu::PresentMode> {
    if vsync_enabled {
        return present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .or_else(|| present_modes.first().copied())
            .ok_or_else(|| anyhow!("surface has no present modes"));
    }

    present_modes
        .iter()
        .copied()
        .find(|mode| *mode == wgpu::PresentMode::Immediate)
        .or_else(|| {
            present_modes
                .iter()
                .copied()
                .find(|mode| *mode == wgpu::PresentMode::Mailbox)
        })
        .or_else(|| {
            present_modes
                .iter()
                .copied()
                .find(|mode| *mode != wgpu::PresentMode::Fifo)
        })
        .or_else(|| present_modes.first().copied())
        .ok_or_else(|| anyhow!("surface has no present modes"))
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        Self::new_with_vsync(window, true).await
    }

    pub async fn new_with_vsync(window: &Window, vsync_enabled: bool) -> Result<Self> {
        Self::new_with_vsync_and_terrain_filtering(
            window,
            vsync_enabled,
            TerrainTextureFiltering::None,
        )
        .await
    }

    pub async fn new_with_vsync_and_terrain_filtering(
        window: &Window,
        vsync_enabled: bool,
        terrain_texture_filtering: TerrainTextureFiltering,
    ) -> Result<Self> {
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
        let adapter_info = adapter.get_info();
        tracing::info!(
            adapter_name = %adapter_info.name,
            adapter_backend = %adapter_info.backend.to_str(),
            adapter_device_type = ?adapter_info.device_type,
            "renderer platform adapter selected"
        );

        let required_features = optional_gpu_timing_features(adapter.features());
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("bbb-native-device"),
                    required_features,
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = choose_format(&caps.formats)?;
        let present_mode = choose_present_mode(&caps.present_modes, vsync_enabled)?;
        tracing::info!(
            ?present_mode,
            supported_present_modes = ?caps.present_modes,
            desired_maximum_frame_latency = 2,
            "renderer surface presentation selected"
        );
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
            // Keep wgpu's default queue-depth hint. VSync-off selection still maps to the
            // platform's immediate present path, while a larger hint did not improve the
            // common-X11 benchmark and would add an unproven behavioral variable.
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let present_modes = caps.present_modes;
        let mut renderer = Self::with_gpu(
            RenderSurface::Window(surface),
            device,
            queue,
            config,
            size,
            terrain_texture_filtering,
        )?;
        renderer.present_modes = present_modes;
        renderer.vsync_enabled = vsync_enabled;
        Ok(renderer)
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
        terrain_texture_filtering: TerrainTextureFiltering,
    ) -> Result<Self> {
        let format = config.format;
        let screen_blur_bind_group_layout = create_screen_blur_bind_group_layout(&device);
        let screen_blur_horizontal_pipeline =
            create_screen_blur_horizontal_pipeline(&device, format, &screen_blur_bind_group_layout);
        let screen_blur_vertical_pipeline =
            create_screen_blur_vertical_pipeline(&device, format, &screen_blur_bind_group_layout);
        let screen_blur_target = create_screen_blur_target(
            &device,
            &screen_blur_bind_group_layout,
            format,
            config.width,
            config.height,
        );
        let camera_post_effect = CameraPostEffectGpu::new(
            &device,
            format,
            config.width,
            config.height,
            &screen_blur_bind_group_layout,
        );
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
        let terrain_section_bind_group_layout = create_terrain_section_bind_group_layout(&device);
        let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
        let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
        let lightmap_pipeline =
            create_lightmap_pipeline(&device, &lightmap_bind_group_layout, None)?;
        let lightmap = create_lightmap_gpu(
            &device,
            &queue,
            &lightmap_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            LightmapEnvironment::default(),
        );
        let hud_bind_group_layout = create_hud_bind_group_layout(&device);
        let lightmap_hud_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bbb-lightmap-hud-preview-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let lightmap_hud_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bbb-lightmap-hud-preview-bind-group"),
            layout: &hud_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lightmap.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&lightmap_hud_sampler),
                },
            ],
        });
        let camera_buffer = create_camera_buffer(&device);
        let hud3d_camera_buffer = create_camera_buffer(&device);
        let gui_item_camera_buffer = create_camera_buffer(&device);
        let terrain_atlas = create_terrain_atlas_gpu(&device, &queue, 1, 1, &[255, 255, 255, 255])?;
        let terrain_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &camera_buffer,
            &terrain_atlas,
        );
        let hud3d_terrain_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &hud3d_camera_buffer,
            &terrain_atlas,
        );
        let hud_terrain_atlas_bind_group = {
            let (view, sampler) = terrain_atlas.view_and_sampler();
            crate::hud::create_hud_atlas_bind_group(
                &device,
                &hud_bind_group_layout,
                "bbb-hud-terrain-atlas-bind-group",
                view,
                sampler,
            )
        };
        // The GUI item pass reuses the item-model pipeline + blocks atlas but with its own ortho camera.
        let gui_item_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &gui_item_camera_buffer,
            &terrain_atlas,
        );
        let terrain_solid_pipeline = create_terrain_solid_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            &terrain_section_bind_group_layout,
            terrain_texture_filtering,
        );
        let terrain_cutout_pipeline = create_terrain_cutout_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            &terrain_section_bind_group_layout,
            terrain_texture_filtering,
        );
        let terrain_translucent_pipeline = create_terrain_translucent_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            &terrain_section_bind_group_layout,
            terrain_texture_filtering,
        );
        let block_destroy_pipeline =
            create_block_destroy_pipeline(&device, format, &terrain_bind_group_layout)?;
        let debug_filled_box_pipeline =
            create_debug_filled_box_pipeline(&device, format, &terrain_bind_group_layout);
        let debug_text_pipeline = create_debug_text_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
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
        let entity_model_solid_pipeline = create_entity_model_solid_pipeline(
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
        let entity_model_solid_z_offset_forward_pipeline =
            create_entity_model_solid_z_offset_forward_pipeline(
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
        let entity_model_armor_decal_pipeline = create_entity_model_armor_decal_pipeline(
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
        let entity_model_banner_pattern_pipeline = create_entity_model_banner_pattern_pipeline(
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
        let entity_model_dragon_rays_pipeline = create_entity_model_dragon_rays_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_dragon_rays_depth_pipeline =
            create_entity_model_dragon_rays_depth_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                None,
            )?;
        let entity_model_leash_pipeline = create_entity_model_leash_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let entity_model_end_portal_pipeline = create_entity_model_end_portal_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_end_gateway_pipeline = create_entity_model_end_gateway_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let hud_win_portal_pipeline = create_win_screen_end_portal_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_water_mask_pipeline = create_entity_model_water_mask_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_outline_pipeline = create_entity_model_outline_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_outline_cull_pipeline = create_entity_model_outline_cull_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_scroll_pipeline = create_entity_model_scroll_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let entity_model_scroll_additive_pipeline = create_entity_model_scroll_additive_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_guardian_beam_pipeline = create_entity_model_guardian_beam_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let entity_model_end_crystal_beam_pipeline = create_entity_model_end_crystal_beam_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let entity_model_beacon_beam_opaque_pipeline =
            create_entity_model_beacon_beam_opaque_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                None,
            )?;
        let entity_model_beacon_beam_translucent_pipeline =
            create_entity_model_beacon_beam_translucent_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                None,
            )?;
        let entity_model_entity_glint_pipeline = create_entity_model_entity_glint_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let entity_model_armor_entity_glint_pipeline =
            create_entity_model_armor_entity_glint_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                None,
            )?;
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
            create_lightning_pipeline(&device, format, &terrain_bind_group_layout, None)?;
        let world_border_pipeline =
            create_world_border_pipeline(&device, format, &terrain_bind_group_layout)?;
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
        let entity_shadow_pipeline = create_entity_shadow_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let item_model_z_offset_forward_pipeline = create_item_model_z_offset_forward_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let item_model_translucent_pipeline = create_item_model_translucent_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let item_model_glint_pipeline =
            create_item_model_glint_pipeline(&device, format, &terrain_bind_group_layout, None)?;
        let display_text_background_pipeline = create_display_text_background_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let display_text_pipeline = create_display_text_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let display_text_see_through_pipeline = create_display_text_see_through_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let resident_item_model_pipeline = create_resident_item_model_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
        let resident_item_model_solid_no_cutout_pipeline =
            create_resident_item_model_solid_no_cutout_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
                None,
            )?;
        let resident_item_model_z_offset_forward_pipeline =
            create_resident_item_model_z_offset_forward_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
                None,
            )?;
        let resident_item_model_cutout_no_cull_pipeline =
            create_resident_item_model_cutout_no_cull_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
                None,
            )?;
        let resident_item_model_cutout_z_offset_pipeline =
            create_resident_item_model_cutout_z_offset_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
                None,
            )?;
        let resident_item_model_pattern_pipeline = create_resident_item_model_pattern_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )?;
        let resident_item_model_translucent_pipeline =
            create_resident_item_model_translucent_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
            );
        let resident_item_model_translucent_no_cull_pipeline =
            create_resident_item_model_translucent_no_cull_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
                None,
            )?;
        let resident_item_model_glint_pipeline = create_resident_item_model_glint_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let resident_item_model_end_portal_pipeline =
            create_resident_item_model_end_portal_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                None,
            )?;
        let resident_item_model_end_gateway_pipeline =
            create_resident_item_model_end_gateway_pipeline(
                &device,
                format,
                &terrain_bind_group_layout,
                None,
            )?;
        let resident_item_model_outline_pipeline = create_resident_item_model_outline_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let hud_item_glint_pipeline = create_hud_item_glint_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &hud_bind_group_layout,
            None,
        )?;
        let selection_pipeline =
            create_selection_pipeline(&device, format, &terrain_bind_group_layout, None)?;
        let translucent_debug_line_pipeline = create_translucent_debug_line_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
        let chunk_border_always_on_top_pipeline = create_chunk_border_always_on_top_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            None,
        )?;
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
            None,
        )?;
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
            None,
        )?;
        let sunrise_sunset_pipeline =
            create_sunrise_sunset_pipeline(&device, format, &terrain_bind_group_layout);
        let star_pipeline = create_star_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &sky_dynamic_bind_group_layout,
            None,
        )?;
        let end_sky_texture_bind_group_layout = create_end_sky_bind_group_layout(&device);
        let end_sky_pipeline = create_end_sky_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &end_sky_texture_bind_group_layout,
            &sky_dynamic_bind_group_layout,
            None,
        )?;
        let end_sky_mesh = create_end_sky_gpu(&device, &sky_dynamic_bind_group_layout);
        let celestial_bind_group_layout = create_celestial_bind_group_layout(&device);
        let celestial_pipeline = create_celestial_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &celestial_bind_group_layout,
            &sky_dynamic_bind_group_layout,
            None,
        )?;
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
            None,
        )?;
        let cloud_fancy_pipeline = create_cloud_pipeline(
            &device,
            format,
            &terrain_bind_group_layout,
            &cloud_bind_group_layout,
            CloudShape::Fancy,
            None,
        )?;
        let hud_pipeline = create_hud_pipeline(&device, format, &hud_bind_group_layout, None)?;
        let hud_nausea_overlay_pipeline =
            create_hud_nausea_overlay_pipeline(&device, format, &hud_bind_group_layout, None)?;
        let vignette_bind_group_layout = create_vignette_bind_group_layout(&device);
        let vignette_pipeline =
            create_vignette_pipeline(&device, format, &vignette_bind_group_layout, None)?;
        let screen_effect_pipeline = crate::screen_effects::create_screen_effect_pipeline(
            &device,
            format,
            &hud_bind_group_layout,
            None,
        )?;
        let map_surface_residency = MapSurfaceResidency::new(
            &device,
            format,
            &terrain_bind_group_layout,
            &lightmap_sample_bind_group_layout,
        );
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
        let gpu_timing = GpuFrameTiming::new(&device, &queue);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            present_modes: Vec::new(),
            vsync_enabled: true,
            gui_surface_size: size,
            #[cfg(test)]
            camera_uniform_writes: std::cell::Cell::new(0),
            size,
            clear: ClearColor::default(),
            level_frame_route: LevelFrameRoute::default(),
            counters: RendererCounters {
                width: size.width,
                height: size.height,
                ..RendererCounters::default()
            },
            #[cfg(feature = "lifecycle-soak")]
            lifecycle_soak_surface_faults: std::collections::VecDeque::new(),
            started_at: Instant::now(),
            gpu_timing,
            last_frame_profile: RendererFrameProfile::default(),
            last_frame_started_at: Instant::now(),
            shader_game_time_ticks: 0.0,
            glint_speed: VANILLA_DEFAULT_GLINT_SPEED,
            glint_alpha: crate::camera::VANILLA_DEFAULT_GLINT_STRENGTH,
            current_post_effect_id: None,
            camera_post_effect_active: false,
            camera_post_effect,
            post_effect_graphs: None,
            menu_background_blurriness: sanitize_menu_background_blurriness(
                VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
            ),
            hud_text_background_opacity: crate::hud::VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY,
            hud_background_for_chat_only: true,
            hide_gui: false,
            hud_screen_blur: false,
            screen_blur_bind_group_layout,
            screen_blur_horizontal_pipeline,
            screen_blur_vertical_pipeline,
            screen_blur_target,
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
            terrain_solid_pipeline,
            terrain_cutout_pipeline,
            terrain_translucent_pipeline,
            core_shader_generation: None,
            terrain_texture_filtering,
            // Vanilla `maxAnisotropyBit` default (`IntRange(1, 3)`, 2 → 4x);
            // consumed by the terrain sampler only under `Anisotropic`.
            terrain_max_anisotropy_bit: 2,
            block_destroy_pipeline,
            debug_filled_box_pipeline,
            debug_text_pipeline,
            entity_model_pipeline,
            entity_model_textured_pipeline,
            entity_model_textured_cull_pipeline,
            entity_model_solid_pipeline,
            entity_model_dissolve_pipeline,
            entity_model_solid_z_offset_forward_pipeline,
            entity_model_cutout_z_offset_pipeline,
            entity_model_armor_cutout_pipeline,
            entity_model_armor_decal_pipeline,
            entity_model_translucent_pipeline,
            entity_model_banner_pattern_pipeline,
            entity_model_translucent_cull_pipeline,
            entity_model_armor_translucent_pipeline,
            entity_model_translucent_emissive_pipeline,
            entity_model_eyes_pipeline,
            entity_model_dragon_rays_pipeline,
            entity_model_dragon_rays_depth_pipeline,
            entity_model_leash_pipeline,
            entity_model_end_portal_pipeline,
            entity_model_end_gateway_pipeline,
            hud_win_portal_pipeline,
            entity_model_water_mask_pipeline,
            entity_model_outline_pipeline,
            entity_model_outline_cull_pipeline,
            entity_model_scroll_pipeline,
            entity_model_scroll_additive_pipeline,
            entity_model_guardian_beam_pipeline,
            entity_model_end_crystal_beam_pipeline,
            entity_model_beacon_beam_opaque_pipeline,
            entity_model_beacon_beam_translucent_pipeline,
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
            frame_debug_filled_box_vertices: FrameDataBuffer::vertex(
                "bbb-debug-filled-box-frame-vertices",
            ),
            frame_debug_filled_box_indices: FrameDataBuffer::index(
                "bbb-debug-filled-box-frame-indices",
            ),
            frame_debug_text_vertices: FrameDataBuffer::vertex("bbb-debug-text-frame-vertices"),
            frame_debug_text_indices: FrameDataBuffer::index("bbb-debug-text-frame-indices"),
            frame_debug_always_on_top_text_vertices: FrameDataBuffer::vertex(
                "bbb-debug-always-on-top-text-frame-vertices",
            ),
            frame_debug_always_on_top_text_indices: FrameDataBuffer::index(
                "bbb-debug-always-on-top-text-frame-indices",
            ),
            frame_debug_line_opaque_vertices: FrameDataBuffer::vertex(
                "bbb-debug-line-opaque-frame-vertices",
            ),
            frame_debug_line_translucent_vertices: FrameDataBuffer::vertex(
                "bbb-debug-line-translucent-frame-vertices",
            ),
            frame_entity_model_line_vertices: FrameDataBuffer::vertex(
                "bbb-entity-model-line-frame-vertices",
            ),
            frame_block_entity_crumbling_vertices: FrameDataBuffer::vertex(
                "bbb-block-entity-crumbling-frame-vertices",
            ),
            frame_block_entity_crumbling_indices: FrameDataBuffer::index(
                "bbb-block-entity-crumbling-frame-indices",
            ),
            frame_item_entity_vertices: FrameDataBuffer::vertex("bbb-item-entity-frame-vertices"),
            frame_hud_vertices: FrameDataBuffer::vertex("bbb-hud-frame-vertices"),
            weather_pipeline,
            lightning_pipeline,
            world_border_pipeline,
            item_entity_pipeline,
            item_model_pipeline,
            entity_shadow_pipeline,
            item_model_z_offset_forward_pipeline,
            item_model_translucent_pipeline,
            item_model_glint_pipeline,
            display_text_background_pipeline,
            display_text_pipeline,
            display_text_see_through_pipeline,
            resident_item_model_pipeline,
            resident_item_model_solid_no_cutout_pipeline,
            resident_item_model_z_offset_forward_pipeline,
            resident_item_model_cutout_no_cull_pipeline,
            resident_item_model_cutout_z_offset_pipeline,
            resident_item_model_pattern_pipeline,
            resident_item_model_translucent_pipeline,
            resident_item_model_translucent_no_cull_pipeline,
            resident_item_model_glint_pipeline,
            resident_item_model_end_portal_pipeline,
            resident_item_model_end_gateway_pipeline,
            resident_item_model_outline_pipeline,
            hud_item_glint_pipeline,
            selection_pipeline,
            translucent_debug_line_pipeline,
            chunk_border_always_on_top_pipeline,
            lightmap_pipeline,
            lightmap,
            lightmap_dirty: true,
            lightmap_bind_group_layout,
            lightmap_hud_bind_group,
            _lightmap_hud_sampler: lightmap_hud_sampler,
            lightmap_sample_bind_group_layout,
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
            cloud_bind_group_layout,
            cloud_bind_group,
            cloud_uniform_buffer,
            hud_pipeline,
            hud_nausea_overlay_pipeline,
            hud_bind_group_layout,
            vignette_pipeline,
            vignette_bind_group_layout,
            screen_effect_pipeline,
            screen_effect_quads: Vec::new(),
            screen_effect_underwater: None,
            hud_white_pixel,
            hud_black_pixel,
            terrain_bind_group_layout,
            terrain_section_bind_group_layout,
            camera_buffer,
            hud3d_camera_buffer,
            gui_item_camera_buffer,
            terrain_atlas,
            terrain_atlas_animation: None,
            terrain_bind_group,
            hud3d_terrain_bind_group,
            hud_terrain_atlas_bind_group,
            gui_item_bind_group,
            hud_hotbar_block_item_models: Vec::new(),
            hud_offhand_slot: None,
            hud_offhand_block_item_model: None,
            hud_hotbar_attack_indicator: crate::hud::HudHotbarAttackIndicator::None,
            terrain_opaque: Vec::new(),
            terrain_cutout: Vec::new(),
            terrain_translucent: Vec::new(),
            terrain_section_revisions: TerrainSectionMap::default(),
            terrain_section_visibility: TerrainSectionMap::default(),
            terrain_next_section_visibility: TerrainSectionMap::default(),
            terrain_section_visibility_has_fading: false,
            #[cfg(test)]
            terrain_section_uniform_scan_count: 0,
            terrain_visible_section_order: Vec::new(),
            terrain_next_visible_section_order: Vec::new(),
            terrain_nearby_visible_section_order: Vec::new(),
            terrain_next_nearby_visible_section_order: Vec::new(),
            terrain_translucent_indices: TerrainSectionMap::default(),
            terrain_section_states_revision: None,
            terrain_translucent_order: Vec::new(),
            last_translucent_sort_block_position: None,
            translucency_resort_iteration_index: 0,
            translucency_resort_drain_index: 0,
            terrain_source_sections: 0,
            terrain_bounds: None,
            entity_model_bounds: None,
            camera_pose: None,
            camera_hurt_effect: None,
            camera_view_bob_effect: None,
            camera_spinning_effect: None,
            camera_fov_degrees: 70.0,
            camera_hud_fov_degrees: 70.0,
            lightmap_environment: LightmapEnvironment::default(),
            fog_environment: FogEnvironment::default(),
            sky_environment: SkyEnvironment::default(),
            cloud_environment: CloudEnvironment::default(),
            cloud_frame: CloudFrame::default(),
            cloud_shape: CloudShape::default(),
            clouds_enabled: true,
            sky_disc: None,
            end_sky_mesh,
            end_sky_texture: None,
            sky_celestials: None,
            sky_stars: None,
            celestial_atlas: None,
            cloud_texture: None,
            clouds: None,
            block_destroy_overlays: None,
            block_entity_crumbling_index_count: 0,
            entity_model_mesh: None,
            entity_model_water_mask_mesh: None,
            entity_model_solid_mesh: None,
            entity_model_textured_mesh: None,
            entity_model_textured_cull_mesh: None,
            entity_model_dissolve_mesh: None,
            entity_model_solid_z_offset_forward_mesh: None,
            entity_model_cutout_z_offset_mesh: None,
            entity_model_armor_cutout_mesh: None,
            entity_model_armor_trim_mesh: None,
            entity_model_armor_trim_decal_mesh: None,
            entity_model_translucent_mesh: None,
            entity_model_armor_translucent_mesh: None,
            entity_model_translucent_emissive_mesh: None,
            entity_model_item_entity_translucent_mesh: None,
            entity_model_item_entity_translucent_cull_mesh: None,
            entity_model_opaque_beam_draws: Vec::new(),
            entity_model_sorted_main_translucent_draws: Vec::new(),
            entity_model_sorted_translucent_draws: Vec::new(),
            entity_model_sorted_item_entity_draws: Vec::new(),
            entity_model_eyes_mesh: None,
            entity_model_dragon_rays_mesh: None,
            entity_model_leash_mesh: None,
            entity_model_dragon_rays_depth_mesh: None,
            entity_model_end_portal_mesh: None,
            entity_model_end_gateway_mesh: None,
            hud_win_portal_mesh: None,
            entity_model_outline_mesh: None,
            entity_model_outline_cull_mesh: None,
            entity_model_entity_glint_mesh: None,
            entity_model_armor_entity_glint_mesh: None,
            entity_dynamic_player_skin_solid_mesh: None,
            entity_dynamic_player_skin_cutout_mesh: None,
            entity_dynamic_player_skin_cutout_cull_mesh: None,
            entity_dynamic_player_skin_solid_z_offset_forward_mesh: None,
            entity_dynamic_player_skin_cutout_z_offset_mesh: None,
            entity_dynamic_player_skin_translucent_mesh: None,
            entity_dynamic_player_skin_item_entity_translucent_mesh: None,
            entity_dynamic_player_skin_item_entity_translucent_cull_mesh: None,
            entity_dynamic_player_texture_cutout_mesh: None,
            entity_dynamic_player_texture_solid_mesh: None,
            entity_dynamic_player_texture_cutout_cull_mesh: None,
            entity_dynamic_player_texture_solid_z_offset_forward_mesh: None,
            entity_dynamic_player_texture_cutout_z_offset_mesh: None,
            entity_dynamic_player_texture_armor_cutout_mesh: None,
            entity_dynamic_player_texture_armor_trim_mesh: None,
            entity_dynamic_player_texture_armor_trim_decal_mesh: None,
            entity_dynamic_player_texture_translucent_mesh: None,
            entity_dynamic_player_texture_item_entity_translucent_mesh: None,
            entity_dynamic_player_texture_item_entity_translucent_cull_mesh: None,
            first_person_player_arm_mesh: None,
            first_person_dynamic_player_arm_mesh: None,
            entity_model_scroll_mesh: None,
            entity_model_scroll_additive_mesh: None,
            entity_model_guardian_beam_mesh: None,
            entity_model_end_crystal_beam_mesh: None,
            entity_model_beacon_beam_opaque_mesh: None,
            entity_model_beacon_beam_translucent_mesh: None,
            entity_model_texture_atlas: None,
            entity_dynamic_player_skin_atlas: None,
            entity_dynamic_player_skin_images: Vec::new(),
            entity_dynamic_player_texture_atlas: None,
            entity_dynamic_player_texture_images: Vec::new(),
            dynamic_entity_texture_generation: None,
            entity_model_instances: Vec::new(),
            entity_flame_surfaces: Vec::new(),
            entity_shadow_surfaces: Vec::new(),
            entity_shadow_texture: None,
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
            armor_glint_texture: None,
            item_entity_billboards: Vec::new(),
            item_model_residency: ItemModelResidency::new(),
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
            #[cfg(test)]
            item_frame_map_surfaces: Vec::new(),
            #[cfg(test)]
            first_person_map_surfaces: Vec::new(),
            #[cfg(test)]
            first_person_map_background_surfaces: Vec::new(),
            first_person_map_background_atlas: None,
            item_frame_map_atlas: None,
            #[cfg(test)]
            item_frame_map_decoration_surfaces: Vec::new(),
            #[cfg(test)]
            first_person_map_decoration_surfaces: Vec::new(),
            item_frame_map_decoration_atlas: None,
            sign_text_surfaces: Vec::new(),
            visible_sign_text_positions: Vec::new(),
            sign_text_outline_context: SignTextOutlineContext::default(),
            display_text_surfaces: Vec::new(),
            entity_name_tag_surfaces: Vec::new(),
            item_frame_map_text_font_atlas: None,
            map_surface_residency,
            block_selection: None,
            chunk_border_outline: None,
            entity_scene_outline: None,
            debug_filled_boxes: Vec::new(),
            debug_filled_rects: Vec::new(),
            debug_text_billboards: Vec::new(),
            debug_lines: Vec::new(),
            game_test_highlight_boxes: Vec::new(),
            game_test_highlight_text_billboards: Vec::new(),
            hud_spyglass_scope_texture: None,
            hud_camera_overlay_textures: HashMap::new(),
            hud_camera_overlays: Vec::new(),
            hud_portal_overlay: None,
            hud_confusion_overlay: None,
            hud_nausea_overlay_texture: None,
            hud_crosshair: None,
            hud_crosshair_visible: true,
            hud_crosshair_attack_indicator_full: None,
            hud_crosshair_attack_indicator_background: None,
            hud_crosshair_attack_indicator_progress: None,
            hud_crosshair_attack_indicator: HudCrosshairAttackIndicator::None,
            hud_hotbar: None,
            hud_hotbar_selection: None,
            hud_hotbar_offhand_left: None,
            hud_hotbar_offhand_right: None,
            hud_hotbar_attack_indicator_background: None,
            hud_hotbar_attack_indicator_progress: None,
            hud_spectator_close: None,
            hud_spectator_scroll_left: None,
            hud_spectator_scroll_right: None,
            hud_spectator_teleport_to_player: None,
            hud_spectator_teleport_to_team: None,
            hud_default_player_faces: HashMap::new(),
            hud_dynamic_player_faces: HashMap::new(),
            hud_player_list_ping_sprites: std::array::from_fn(|_| None),
            hud_social_interactions_background: None,
            hud_social_interactions_search: None,
            hud_social_interactions_mute: None,
            hud_social_interactions_mute_highlighted: None,
            hud_social_interactions_unmute: None,
            hud_social_interactions_unmute_highlighted: None,
            hud_item_atlas: None,
            hud_digit_atlas: None,
            hud_digit_glyphs: [HudDigitGlyph::default(); 10],
            hud_font_atlas: None,
            hud_font_glyphs: HudFontGlyphMap::default(),
            hud_font_book: HudFontGlyphBook::default(),
            hud_obfuscated_glyph_pool: HudObfuscatedGlyphPool::default(),
            hud_chat_modified_icon: None,
            hud_draft_report_icon: None,
            hud_hotbar_item_icons: std::array::from_fn(|_| None),
            hud_inworld_menu_background: None,
            hud_inventory_background: None,
            hud_creative_items_background: None,
            hud_creative_item_search_background: None,
            hud_creative_inventory_background: None,
            hud_creative_scroller: None,
            hud_creative_scroller_disabled: None,
            hud_creative_top_unselected_tabs: std::array::from_fn(|_| None),
            hud_creative_top_selected_tabs: std::array::from_fn(|_| None),
            hud_creative_bottom_unselected_tabs: std::array::from_fn(|_| None),
            hud_creative_bottom_selected_tabs: std::array::from_fn(|_| None),
            hud_tooltip_background: None,
            hud_tooltip_frame: None,
            hud_tooltip_styles: HashMap::new(),
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
            hud_widget_preedit: None,
            hud_widget_button: None,
            hud_widget_button_disabled: None,
            hud_widget_button_highlighted: None,
            hud_dialog_warning_button: None,
            hud_dialog_warning_button_disabled: None,
            hud_dialog_warning_button_highlighted: None,
            hud_widget_checkbox: None,
            hud_widget_checkbox_highlighted: None,
            hud_widget_checkbox_selected: None,
            hud_widget_checkbox_selected_highlighted: None,
            hud_widget_slider: None,
            hud_widget_slider_highlighted: None,
            hud_widget_slider_handle: None,
            hud_widget_slider_handle_highlighted: None,
            hud_stats_sprites: std::array::from_fn(|_| None),
            hud_stats_scrollbar_sprites: std::array::from_fn(|_| None),
            hud_system_toast_background: None,
            hud_system_toast: None,
            hud_advancement_toast_background: None,
            hud_advancement_toasts: Vec::new(),
            hud_recipe_toast_background: None,
            hud_recipe_toasts: Vec::new(),
            hud_tutorial_toast_background: None,
            hud_tutorial_toast_icons: std::array::from_fn(|_| None),
            hud_tutorial_toasts: Vec::new(),
            hud_debug_game_mode_switcher_background: None,
            hud_debug_game_mode_switcher_slot: None,
            hud_debug_game_mode_switcher_selection: None,
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
            hud_mob_effect_sprites: BTreeMap::new(),
            hud_effect_background: None,
            hud_effect_background_ambient: None,
            hud_inventory_effect_background: None,
            hud_inventory_effect_background_ambient: None,
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
            hud_smithing_slot_icons: std::array::from_fn(|_| None),
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
            hud_page_backward_highlighted: None,
            hud_page_forward: None,
            hud_page_forward_highlighted: None,
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
            hud_command_block_editor_screen: None,
            hud_structure_block_editor_screen: None,
            hud_jigsaw_block_editor_screen: None,
            hud_test_block_editor_screen: None,
            hud_test_instance_block_editor_screen: None,
            hud_pause_screen: None,
            hud_social_interactions_screen: None,
            hud_player_reporting_screen: None,
            hud_stats_screen: None,
            hud_game_rules_screen: None,
            hud_death_screen: None,
            hud_disconnected_screen: None,
            hud_level_loading_screen: None,
            hud_game_event_screen: None,
            hud_book_screen: None,
            hud_debug_options_screen: None,
            hud_sound_subtitles: crate::hud::HudSoundSubtitles::default(),
            hud_panorama: None,
            hud_menu_background: None,
            hud_entity_preview_pip_targets: Vec::new(),
            hud_experience_background: None,
            hud_experience_progress: None,
            hud_jump_bar_background: None,
            hud_jump_bar_progress: None,
            hud_jump_bar_cooldown: None,
            hud_locator_background: None,
            hud_locator_arrow_up: None,
            hud_locator_arrow_down: None,
            hud_locator_marker_sprites: BTreeMap::new(),
            hud_locator_styles: BTreeMap::new(),
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
            hud_code_of_conduct_screen: None,
            hud_player_health: None,
            hud_sleep_overlay: None,
            hud_food: None,
            hud_food_effect: crate::hud::HudFoodEffect::default(),
            hud_mob_effects: Vec::new(),
            hud_inventory_shows_mob_effects: false,
            hud_armor: None,
            hud_air: None,
            hud_vehicle_health: None,
            hud_vignette: None,
            hud_spyglass_scope: None,
            hud_vignette_texture: None,
            hud_win_logo: None,
            hud_win_easter_egg_logo: None,
            hud_win_edition: None,
            hud_win_credits_vignette: None,
            hud_demo_popup_background: None,
            hud_contextual_bar: crate::hud::HudContextualBar::Empty,
            hud_experience_level: None,
            hud_hotbar_mode: HudHotbarMode::Ordinary,
            hud_selected_slot: 0,
            hud_action_bar_text: None,
            hud_selected_item_name: None,
            hud_title_text: None,
            hud_scoreboard_sidebar: None,
            hud_player_list: None,
            hud_player_list_runtime: crate::hud::HudPlayerListRuntime::default(),
            hud_chat_messages: Vec::new(),
            hud_chat_lines: Vec::new(),
            hud_chat_context: HudChatContext::default(),
            hud_chat_restrictions_screen: None,
            hud_command_suggestions: None,
            hud_debug_overlay: None,
            hud_boss_bars: Vec::new(),
            hud_boss_bar_backgrounds: Default::default(),
            hud_boss_bar_progress_sprites: Default::default(),
            hud_boss_bar_notched_backgrounds: Default::default(),
            hud_boss_bar_notched_progress_sprites: Default::default(),
            particles: ParticleRuntimeState::default(),
            resource_reload_checkpoint: None,
        })
    }

    fn prepare_resize(
        &self,
        size: PhysicalSize<u32>,
        mut checkpoint: impl FnMut(RendererResizePrepareStage) -> Result<()>,
    ) -> Result<RendererResizePlan> {
        let post_effect_graphs = self
            .post_effect_graphs
            .as_ref()
            .map(|graphs| graphs.prepare_resize(&self.device, size.width, size.height))
            .transpose()?;
        checkpoint(RendererResizePrepareStage::PostEffects)?;

        let plan = crate::post_effect_graph::capture_gpu_errors(&self.device, || {
            let screen_blur_target = create_screen_blur_target(
                &self.device,
                &self.screen_blur_bind_group_layout,
                self.config.format,
                size.width,
                size.height,
            );
            let camera_post_effect = self.camera_post_effect.prepare_resize(
                &self.device,
                size.width,
                size.height,
                &self.screen_blur_bind_group_layout,
            );
            let main_target =
                create_main_target(&self.device, self.config.format, size.width, size.height);
            let depth = create_depth_target(&self.device, size.width, size.height);
            let translucent_target = create_translucent_target(
                &self.device,
                self.config.format,
                size.width,
                size.height,
            );
            let item_entity_target = create_item_entity_target(
                &self.device,
                self.config.format,
                size.width,
                size.height,
            );
            let particle_target =
                create_particle_target(&self.device, self.config.format, size.width, size.height);
            let weather_target =
                create_weather_target(&self.device, self.config.format, size.width, size.height);
            let transparency_final_target = create_transparency_final_target(
                &self.device,
                &self.transparency_blit_bind_group_layout,
                self.config.format,
                size.width,
                size.height,
            );
            let entity_outline_target = create_entity_outline_target(
                &self.device,
                &self.entity_outline_bind_group_layout,
                self.config.format,
                size.width,
                size.height,
            );
            let cloud_target =
                create_cloud_target(&self.device, self.config.format, size.width, size.height);
            let transparency_combine_bind_group = create_transparency_combine_bind_group(
                &self.device,
                &self.transparency_combine_bind_group_layout,
                &main_target,
                &depth,
                &translucent_target,
                &item_entity_target,
                &particle_target,
                &weather_target,
                &cloud_target,
            );
            Ok(RendererResizePlan {
                size,
                post_effect_graphs,
                screen_blur_target,
                camera_post_effect,
                main_target,
                depth,
                translucent_target,
                item_entity_target,
                particle_target,
                weather_target,
                transparency_final_target,
                entity_outline_target,
                cloud_target,
                transparency_combine_bind_group,
            })
        })?;
        checkpoint(RendererResizePrepareStage::FrameTargets)?;
        Ok(plan)
    }

    fn apply_resize(&mut self, plan: RendererResizePlan) {
        let mut config = self.config.clone();
        config.width = plan.size.width;
        config.height = plan.size.height;
        self.surface.configure(&self.device, &config);
        if let (Some(graphs), Some(resize)) =
            (&mut self.post_effect_graphs, plan.post_effect_graphs)
        {
            graphs.apply_resize(resize);
        }
        self.size = plan.size;
        self.config = config;
        self.screen_blur_target = plan.screen_blur_target;
        self.camera_post_effect
            .apply_resize(plan.camera_post_effect);
        self.main_target = plan.main_target;
        self.depth = plan.depth;
        self.translucent_target = plan.translucent_target;
        self.item_entity_target = plan.item_entity_target;
        self.particle_target = plan.particle_target;
        self.weather_target = plan.weather_target;
        self.transparency_final_target = plan.transparency_final_target;
        self.entity_outline_target = plan.entity_outline_target;
        self.cloud_target = plan.cloud_target;
        self.transparency_combine_bind_group = plan.transparency_combine_bind_group;
        self.update_camera();
        self.counters.surface_resizes = self.counters.surface_resizes.saturating_add(1);
        self.counters.width = plan.size.width;
        self.counters.height = plan.size.height;
    }

    fn resize_with_prepare_check(
        &mut self,
        size: PhysicalSize<u32>,
        checkpoint: impl FnMut(RendererResizePrepareStage) -> Result<()>,
    ) -> Result<()> {
        if size.width == 0 || size.height == 0 {
            self.counters.surface_zero_size_skips =
                self.counters.surface_zero_size_skips.saturating_add(1);
            return Ok(());
        }
        let plan = self.prepare_resize(size, checkpoint)?;
        self.apply_resize(plan);
        Ok(())
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        self.resize_with_prepare_check(size, |_| Ok(()))
    }

    /// `Window.updateVsync` (Window.java:246-249): renegotiates the surface
    /// present mode from the supported set without touching the framebuffer
    /// targets. Every fullscreen transition re-applies the stored preference.
    /// A minimized (zero-size) surface only updates the stored config so the
    /// next `resize` reconfigures with it.
    pub fn set_vsync_enabled(&mut self, vsync_enabled: bool) -> Result<()> {
        self.vsync_enabled = vsync_enabled;
        let present_mode = choose_present_mode(&self.present_modes, vsync_enabled)?;
        if self.config.present_mode == present_mode {
            return Ok(());
        }
        let mut config = self.config.clone();
        config.present_mode = present_mode;
        if self.size.width != 0 && self.size.height != 0 {
            self.surface.configure(&self.device, &config);
        }
        self.config = config;
        Ok(())
    }

    pub fn counters(&self) -> RendererCounters {
        self.counters.clone()
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn inject_lifecycle_soak_surface_error(&mut self, error: LifecycleSoakSurfaceError) {
        self.lifecycle_soak_surface_faults
            .push_back(error.into_surface_error());
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn record_lifecycle_soak_window_resize_request(&mut self) {
        self.counters.window_resize_requests =
            self.counters.window_resize_requests.saturating_add(1);
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn record_lifecycle_soak_window_minimize_request(&mut self) {
        self.counters.window_minimize_requests =
            self.counters.window_minimize_requests.saturating_add(1);
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn record_lifecycle_soak_window_restore_request(&mut self) {
        self.counters.window_restore_requests =
            self.counters.window_restore_requests.saturating_add(1);
    }

    pub fn set_gpu_timing_enabled(&mut self, enabled: bool) {
        if let Some(timing) = self.gpu_timing.as_mut() {
            timing.set_enabled(enabled);
        }
    }

    pub fn set_level_frame_route(&mut self, route: LevelFrameRoute) {
        self.level_frame_route = route;
    }

    pub fn gpu_utilization(&self) -> Option<f64> {
        self.gpu_timing
            .as_ref()
            .and_then(GpuFrameTiming::latest_utilization)
    }

    pub fn last_frame_profile(&self) -> RendererFrameProfile {
        self.last_frame_profile
    }

    pub fn set_terrain_mesh_worker_counters(&mut self, counters: TerrainMeshWorkerCounters) {
        self.counters.terrain_selected_chunks = counters.selected_chunks;
        self.counters.terrain_resident_chunks = counters.resident_chunks;
        self.counters.terrain_selected_sections = counters.selected_sections;
        self.counters.terrain_resident_sections = counters.resident_sections;
        self.counters.terrain_pending_jobs = counters.pending_jobs;
        self.counters.terrain_cache_ready = counters.cache_ready;
        self.counters.terrain_mesh_jobs_queued = counters.queued_jobs;
        self.counters.terrain_mesh_jobs_completed = counters.completed_jobs;
        self.counters.terrain_mesh_jobs_stale = counters.stale_jobs;
        self.counters.terrain_mesh_sections_queued = counters.queued_sections;
        self.counters.terrain_mesh_sections_completed = counters.completed_sections;
        self.counters.terrain_mesh_sections_stale = counters.stale_sections;
        self.counters.terrain_mesh_queue_full = counters.queue_full;
        self.counters.terrain_mesh_queue_depth = counters.queue_depth;
        self.counters.terrain_mesh_peak_queue_depth = counters.peak_queue_depth;
        self.counters.terrain_mesh_recent_build_nanos = counters.recent_build_nanos;
        self.counters.terrain_mesh_peak_build_nanos = counters.peak_build_nanos;
        self.counters.terrain_mesh_recent_build_sections = counters.recent_build_sections;
        self.counters.terrain_mesh_peak_build_sections = counters.peak_build_sections;
    }

    pub fn set_camera_post_effect(&mut self, id: Option<&str>, active: bool) {
        if self.current_post_effect_id.as_deref() == id && self.camera_post_effect_active == active
        {
            return;
        }
        self.current_post_effect_id = id.map(str::to_owned);
        self.camera_post_effect_active = active;
    }

    /// Replaces the frame's vanilla `ScreenEffectRenderer` overlays, in the
    /// vanilla draw order (view-blocking block, then underwater, then fire).
    pub fn set_screen_effect_quads(&mut self, quads: Vec<crate::screen_effects::ScreenEffectQuad>) {
        self.screen_effect_quads = quads;
    }

    /// Uploads the selected pack generation's `textures/misc/underwater.png`
    /// consumed by the first-person underwater screen effect.
    pub fn upload_screen_effect_underwater_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.screen_effect_underwater =
            Some(crate::screen_effects::create_screen_effect_underwater_gpu(
                &self.device,
                &self.queue,
                &self.hud_bind_group_layout,
                width,
                height,
                rgba,
            )?);
        Ok(())
    }

    pub fn current_post_effect_id(&self) -> Option<&str> {
        self.current_post_effect_id.as_deref()
    }

    pub fn camera_post_effect_active(&self) -> bool {
        self.camera_post_effect_active
    }

    /// Sets vanilla's startup-only menu background blur option (`0..=10`).
    /// A value below one disables `Screen.extractBlurredBackground`'s stratum marker.
    pub fn set_menu_background_blurriness(&mut self, blurriness: u8) {
        self.menu_background_blurriness = sanitize_menu_background_blurriness(blurriness);
    }

    /// Sets the startup-only equivalents of Vanilla's accessibility text
    /// background options. Chat always uses the configured opacity; other
    /// `textWithBackdrop` consumers use it only when chat-only mode is off.
    pub fn set_hud_text_background_options(
        &mut self,
        opacity: f32,
        background_for_chat_only: bool,
    ) {
        self.hud_text_background_opacity = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0)
        } else {
            crate::hud::VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY
        };
        self.hud_background_for_chat_only = background_for_chat_only;
    }

    /// Marks whether the projected screen inherited vanilla's non-in-game background.
    pub fn set_hud_screen_blur(&mut self, active: bool) {
        self.hud_screen_blur = active;
    }

    pub fn set_camera_pose(&mut self, pose: Option<CameraPose>) {
        if self.camera_pose == pose {
            return;
        }
        self.camera_pose = pose;
        self.resort_translucent_terrain_for_camera();
        if self.entity_model_texture_atlas.is_some()
            && !self.entity_model_instances.is_empty()
            && self.entity_model_meshes_depend_on_camera()
        {
            self.rebuild_entity_model_meshes();
        }
        self.update_camera();
    }

    /// Replaces every camera-uniform input for one renderer frame and publishes
    /// the final projection once. Standalone setters remain available for
    /// initialization and event-driven updates outside the frame handoff.
    pub fn set_camera_frame(&mut self, frame: RendererCameraFrame) {
        let RendererCameraFrame {
            lightmap_environment,
            fog_environment,
            sky_environment,
            pose,
            hurt_effect,
            view_bob_effect,
            spinning_effect,
            fov_degrees,
            hud_fov_degrees,
            shader_game_time_ticks,
        } = frame;

        self.replace_lightmap_environment(lightmap_environment);
        self.replace_fog_environment(fog_environment);
        self.replace_sky_environment(sky_environment);

        let pose_changed = self.camera_pose != pose;
        self.camera_pose = pose;
        self.camera_hurt_effect = hurt_effect;
        self.camera_view_bob_effect = view_bob_effect;
        self.camera_spinning_effect = spinning_effect;
        self.camera_fov_degrees = sanitize_camera_fov_degrees(fov_degrees);
        self.camera_hud_fov_degrees = sanitize_camera_fov_degrees(hud_fov_degrees);
        self.shader_game_time_ticks = sanitize_shader_game_time_ticks(shader_game_time_ticks);

        if pose_changed {
            self.resort_translucent_terrain_for_camera();
            if self.entity_model_texture_atlas.is_some()
                && !self.entity_model_instances.is_empty()
                && self.entity_model_meshes_depend_on_camera()
            {
                self.rebuild_entity_model_meshes();
            }
        }
        // This call also advances the real-time glint transform while the
        // simulation clock is paused, so every presented frame publishes once
        // even when all extracted values compare equal.
        self.update_camera();
    }

    pub fn set_camera_hurt_effect(&mut self, effect: Option<CameraHurtEffect>) {
        if self.camera_hurt_effect == effect {
            return;
        }
        self.camera_hurt_effect = effect;
        self.update_camera();
    }

    pub fn set_camera_view_bob_effect(&mut self, effect: Option<CameraViewBobEffect>) {
        if self.camera_view_bob_effect == effect {
            return;
        }
        self.camera_view_bob_effect = effect;
        self.update_camera();
    }

    pub fn set_camera_spinning_effect(&mut self, effect: Option<CameraSpinningEffect>) {
        if self.camera_spinning_effect == effect {
            return;
        }
        self.camera_spinning_effect = effect;
        self.update_camera();
    }

    pub fn set_camera_fov_degrees(&mut self, fov_degrees: f32, hud_fov_degrees: f32) {
        let fov_degrees = sanitize_camera_fov_degrees(fov_degrees);
        let hud_fov_degrees = sanitize_camera_fov_degrees(hud_fov_degrees);
        if self.camera_fov_degrees == fov_degrees && self.camera_hud_fov_degrees == hud_fov_degrees
        {
            return;
        }
        self.camera_fov_degrees = fov_degrees;
        self.camera_hud_fov_degrees = hud_fov_degrees;
        self.update_camera();
    }

    pub fn set_shader_game_time_ticks(&mut self, ticks: f64) {
        let ticks = sanitize_shader_game_time_ticks(ticks);
        if (self.shader_game_time_ticks - ticks).abs() <= f64::EPSILON {
            return;
        }
        self.shader_game_time_ticks = ticks;
        self.update_camera();
    }

    pub fn set_clear_color(&mut self, clear: ClearColor) {
        self.clear = clear;
    }

    /// Replaces this frame's `RenderTypes.debugFilledBox()` submissions.
    /// Reversed bounds are normalized; non-finite and zero-volume boxes are
    /// discarded. Passing an empty vector clears all previous geometry.
    pub fn set_debug_filled_boxes(&mut self, boxes: Vec<DebugFilledBox>) {
        let _ = self.replace_debug_filled_boxes(boxes);
    }

    /// Replaces dynamic debug boxes and returns one cleared allocation for the
    /// next extraction frame.
    pub fn replace_debug_filled_boxes(
        &mut self,
        boxes: Vec<DebugFilledBox>,
    ) -> Vec<DebugFilledBox> {
        let mut boxes = sanitize_debug_filled_boxes(boxes);
        if self.debug_filled_boxes == boxes {
            boxes.clear();
            return boxes;
        }
        let mut previous = std::mem::replace(&mut self.debug_filled_boxes, boxes);
        previous.clear();
        previous
    }

    /// Replaces this frame's filled `RectGizmo` submissions. Invalid or
    /// degenerate quads are discarded; an empty vector clears stale geometry.
    pub fn set_debug_filled_rects(&mut self, rects: Vec<DebugFilledRect>) {
        let _ = self.replace_debug_filled_rects(rects);
    }

    /// Replaces dynamic debug rects and returns one cleared allocation for the
    /// next extraction frame.
    pub fn replace_debug_filled_rects(
        &mut self,
        rects: Vec<DebugFilledRect>,
    ) -> Vec<DebugFilledRect> {
        let mut rects = sanitize_debug_filled_rects(rects);
        if self.debug_filled_rects == rects {
            rects.clear();
            return rects;
        }
        let mut previous = std::mem::replace(&mut self.debug_filled_rects, rects);
        previous.clear();
        previous
    }

    /// Replaces this frame's camera-facing `TextGizmo` submissions. Invalid
    /// entries are discarded; an empty vector clears the previous frame.
    pub fn set_debug_text_billboards(&mut self, billboards: Vec<DebugTextBillboard>) {
        let _ = self.replace_debug_text_billboards(billboards);
    }

    /// Replaces dynamic debug text and returns one cleared allocation for the
    /// next extraction frame.
    pub fn replace_debug_text_billboards(
        &mut self,
        billboards: Vec<DebugTextBillboard>,
    ) -> Vec<DebugTextBillboard> {
        let mut billboards = sanitize_debug_text_billboards(billboards);
        if self.debug_text_billboards == billboards {
            billboards.clear();
            return billboards;
        }
        let mut previous = std::mem::replace(&mut self.debug_text_billboards, billboards);
        previous.clear();
        previous
    }

    /// Replaces this frame's gizmo line submissions. Opaque and translucent
    /// lines are grouped into their matching vanilla depth-write pipelines.
    pub fn set_debug_lines(&mut self, lines: Vec<DebugLine>) {
        let _ = self.replace_debug_lines(lines);
    }

    /// Replaces dynamic debug lines and returns one cleared allocation for the
    /// next extraction frame.
    pub fn replace_debug_lines(&mut self, lines: Vec<DebugLine>) -> Vec<DebugLine> {
        let mut lines = sanitize_debug_lines(lines);
        if self.debug_lines == lines {
            lines.clear();
            return lines;
        }
        let mut previous = std::mem::replace(&mut self.debug_lines, lines);
        previous.clear();
        previous
    }

    /// Replaces the stable GameTest lane atomically. These values are
    /// concatenated after dynamic `DebugRenderer` values by the shared mesh
    /// builders, matching `LevelRenderer.extractLevel`.
    pub fn set_game_test_highlights(
        &mut self,
        boxes: Vec<DebugFilledBox>,
        text_billboards: Vec<DebugTextBillboard>,
    ) {
        self.game_test_highlight_boxes = sanitize_debug_filled_boxes(boxes);
        self.game_test_highlight_text_billboards = sanitize_debug_text_billboards(text_billboards);
    }

    pub fn set_block_selection(&mut self, state: Option<BlockSelectionRenderState>) {
        if self
            .block_selection
            .as_ref()
            .map(|selection| &selection.state)
            == state.as_ref()
        {
            return;
        }
        self.block_selection = state.map(|state| create_block_selection_gpu(&self.device, state));
    }

    pub fn set_chunk_border_outline(&mut self, outline: Option<SelectionOutline>) {
        let chunk_border_boxes = outline.as_ref().map_or(0, |outline| {
            outline.boxes.len() + outline.colored_boxes.len()
        });
        self.counters.chunk_border_boxes = chunk_border_boxes;
        if self
            .chunk_border_outline
            .as_ref()
            .map(|selection| &selection.outline)
            == outline.as_ref()
        {
            return;
        }
        self.chunk_border_outline =
            outline.map(|outline| create_selection_outline_gpu(&self.device, outline));
    }

    pub fn set_entity_scene_outline(&mut self, outline: Option<SelectionOutline>) {
        let entity_scene_boxes = outline.as_ref().map_or(0, |outline| {
            outline.boxes.len() + outline.colored_boxes.len()
        });
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
        self.upload_terrain_mesh_layer_refs(
            layers.source_sections,
            &layers.opaque,
            &layers.cutout,
            &layers.translucent,
        );
    }

    /// Replaces terrain residency from borrowed per-chunk mesh caches. The GPU
    /// upload consumes vertex/index bytes synchronously, so native can retain
    /// the CPU meshes and reuse every unchanged chunk on the next rebuild.
    pub fn upload_terrain_mesh_layer_refs<'a>(
        &mut self,
        source_sections: usize,
        opaque: impl IntoIterator<Item = &'a terrain::TerrainMesh>,
        cutout: impl IntoIterator<Item = &'a terrain::TerrainMesh>,
        translucent: impl IntoIterator<Item = &'a terrain::TerrainMesh>,
    ) {
        self.terrain_source_sections = source_sections;
        self.counters.queued_sections = self.terrain_source_sections;
        self.counters.meshed_sections = self.terrain_source_sections;
        self.counters.upload_bytes = 0;
        self.counters.terrain_changed_owners = 0;
        self.counters.terrain_uploaded_owners = 0;
        self.counters.terrain_removed_owners = self
            .terrain_opaque
            .len()
            .saturating_add(self.terrain_cutout.len())
            .saturating_add(self.terrain_translucent.len());
        self.counters.terrain_vertex_upload_bytes = 0;
        self.counters.terrain_index_upload_bytes = 0;
        self.counters.terrain_full_replacements =
            self.counters.terrain_full_replacements.saturating_add(1);
        self.terrain_section_revisions.clear();
        self.terrain_section_visibility.clear();
        self.terrain_next_section_visibility.clear();
        self.terrain_section_visibility_has_fading = false;
        self.terrain_visible_section_order.clear();
        self.terrain_next_visible_section_order.clear();
        self.terrain_nearby_visible_section_order.clear();
        self.terrain_next_nearby_visible_section_order.clear();
        self.terrain_translucent_indices.clear();
        self.terrain_section_states_revision = None;
        self.terrain_opaque.clear();
        self.terrain_cutout.clear();
        self.terrain_translucent.clear();

        for mesh in opaque {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh, None, None) {
                self.terrain_opaque.push(resident);
                self.counters.terrain_uploaded_owners += 1;
            }
        }
        for mesh in cutout {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh, None, None) {
                self.terrain_cutout.push(resident);
                self.counters.terrain_uploaded_owners += 1;
            }
        }
        for mesh in translucent {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh, None, None) {
                self.terrain_translucent.push(resident);
                self.counters.terrain_uploaded_owners += 1;
            }
        }
        self.rebuild_translucent_section_indices();
        // The translucent section set just changed (full replace is the only
        // add/remove path). Rebuild both section order and each borrowed CPU
        // cache mesh's indices for the current camera; unchanged cached meshes
        // may have last been built at an older camera position.
        self.resort_translucent_terrain_for_camera();

        self.update_camera();
        self.refresh_terrain_counters();
    }

    /// Atomically replaces the native section-lifecycle projection. Membership,
    /// near-to-far order and fade visibility travel in one value so the terrain
    /// shader cannot drift from entity/block-entity visibility decisions.
    pub fn set_terrain_section_states(
        &mut self,
        revision: u64,
        sections: &[TerrainSectionRenderState],
    ) {
        if self.terrain_section_states_revision == Some(revision) {
            return;
        }
        let visible = &mut self.terrain_next_section_visibility;
        visible.clear();
        visible.reserve(sections.len());
        let order = &mut self.terrain_next_visible_section_order;
        order.clear();
        order.reserve(sections.len());
        let nearby_order = &mut self.terrain_next_nearby_visible_section_order;
        nearby_order.clear();
        let mut has_fading = false;
        for section in sections {
            if !self.terrain_section_revisions.contains_key(&section.owner) {
                continue;
            }
            let visibility = section.visibility.clamp(0.0, 1.0);
            let Entry::Vacant(entry) = visible.entry(section.owner) else {
                continue;
            };
            entry.insert(visibility);
            has_fading |= visibility < 1.0;
            order.push(section.owner);
            if section.nearby {
                nearby_order.push(section.owner);
            }
        }
        self.terrain_section_states_revision = Some(revision);
        if self.terrain_visible_section_order == *order
            && self.terrain_nearby_visible_section_order == *nearby_order
            && self.terrain_section_visibility == *visible
        {
            debug_assert_eq!(self.terrain_section_visibility_has_fading, has_fading);
            return;
        }
        let update_fading_uniforms = self.terrain_section_visibility_has_fading || has_fading;
        std::mem::swap(
            &mut self.terrain_section_visibility,
            &mut self.terrain_next_section_visibility,
        );
        std::mem::swap(
            &mut self.terrain_visible_section_order,
            &mut self.terrain_next_visible_section_order,
        );
        std::mem::swap(
            &mut self.terrain_nearby_visible_section_order,
            &mut self.terrain_next_nearby_visible_section_order,
        );
        self.terrain_section_visibility_has_fading = has_fading;
        if update_fading_uniforms {
            let visibility = &self.terrain_section_visibility;
            let queue = &self.queue;
            #[cfg(test)]
            let mut scanned_meshes = 0;
            for mesh in self
                .terrain_opaque
                .iter_mut()
                .chain(self.terrain_cutout.iter_mut())
                .chain(self.terrain_translucent.iter_mut())
            {
                #[cfg(test)]
                {
                    scanned_meshes += 1;
                }
                let next = mesh
                    .owner
                    .and_then(|owner| visibility.get(&owner).copied())
                    .unwrap_or(1.0);
                if mesh.section_visibility.to_bits() != next.to_bits() {
                    mesh.section_visibility = next;
                    queue.write_buffer(
                        &mesh._section_uniform_buffer,
                        0,
                        bytemuck::cast_slice(&[next, 0.0, 0.0, 0.0]),
                    );
                }
            }
            #[cfg(test)]
            {
                self.terrain_section_uniform_scan_count += scanned_meshes;
            }
        }
        self.rebuild_translucent_section_order();
        // Residency did not change here, and `visible` was already filtered
        // against the current owner table above. Keep the visibility counter
        // on that hot projection path without rescanning every resident mesh
        // for bytes, bounds and face totals.
        self.counters.visible_sections = if self.terrain_section_revisions.is_empty() {
            self.counters.uploaded_sections
        } else {
            self.terrain_section_visibility.len()
        };
    }

    /// Synchronizes renderer-owned section/layer GPU residency by stable
    /// chunk/section owner and revision. Unchanged sections retain their exact
    /// wgpu buffers even when a sibling section in the chunk changes.
    pub fn synchronize_terrain_section_mesh_layer_refs<'a>(
        &mut self,
        source_sections: usize,
        chunks: impl IntoIterator<
            Item = (
                TerrainChunkMeshOwner,
                &'a [(i32, u64)],
                &'a terrain::TerrainSectionMeshLayers,
                [f32; 3],
            ),
        >,
    ) {
        let chunks: Vec<_> = chunks.into_iter().collect();
        let mut desired_sections = TerrainSectionSet::with_capacity_and_hasher(
            source_sections,
            TerrainMeshOwnerBuildHasher,
        );
        let mut visibility_stale = false;
        for (chunk, revisions, _, _) in &chunks {
            for &(section_y, revision) in *revisions {
                let owner = TerrainSectionMeshOwner {
                    chunk: *chunk,
                    section_y,
                };
                visibility_stale |= self.terrain_section_revisions.get(&owner) != Some(&revision);
                desired_sections.insert(owner);
            }
        }
        visibility_stale |= self.terrain_section_revisions.len() != desired_sections.len();
        if visibility_stale {
            // Vanilla keeps the current SectionOcclusionGraph while
            // `schedulePropagationFrom` incrementally admits newly compiled sections. Preserve
            // that graph result here: dropping it would make the upload frame draw every
            // resident section. Prune owners which left residency now. A newly resident owner
            // remains culled until the next native frame projection publishes membership and
            // fade together; recompiling an existing owner retains its current projection.
            self.terrain_section_visibility
                .retain(|owner, _| desired_sections.contains(owner));
            self.terrain_visible_section_order
                .retain(|owner| desired_sections.contains(owner));
        }

        self.terrain_source_sections = source_sections;
        self.counters.queued_sections = source_sections;
        self.counters.meshed_sections = source_sections;
        self.counters.upload_bytes = 0;
        self.counters.terrain_changed_owners = 0;
        self.counters.terrain_uploaded_owners = 0;
        self.counters.terrain_removed_owners = 0;
        self.counters.terrain_vertex_upload_bytes = 0;
        self.counters.terrain_index_upload_bytes = 0;

        self.terrain_section_revisions
            .retain(|owner, _| desired_sections.contains(owner));
        self.counters.terrain_removed_owners +=
            retain_resident_terrain_sections(&mut self.terrain_opaque, &desired_sections);
        self.counters.terrain_removed_owners +=
            retain_resident_terrain_sections(&mut self.terrain_cutout, &desired_sections);
        self.counters.terrain_removed_owners +=
            retain_resident_terrain_sections(&mut self.terrain_translucent, &desired_sections);

        for (chunk, revisions, layers, mesh_camera_position) in chunks {
            for &(section_y, revision) in revisions {
                let owner = TerrainSectionMeshOwner { chunk, section_y };
                if self.terrain_section_revisions.get(&owner) == Some(&revision) {
                    continue;
                }
                self.counters.terrain_changed_owners += 1;
                self.counters.terrain_removed_owners +=
                    remove_resident_terrain_section(&mut self.terrain_opaque, owner);
                self.counters.terrain_removed_owners +=
                    remove_resident_terrain_section(&mut self.terrain_cutout, owner);
                self.counters.terrain_removed_owners +=
                    remove_resident_terrain_section(&mut self.terrain_translucent, owner);

                if let Some(section) = layers
                    .opaque
                    .iter()
                    .find(|section| section.section_y == section_y)
                {
                    if let Some(resident) = self.create_resident_terrain_mesh(
                        &section.mesh,
                        Some(owner),
                        Some(mesh_camera_position),
                    ) {
                        self.terrain_opaque.push(resident);
                        self.counters.terrain_uploaded_owners += 1;
                    }
                }
                if let Some(section) = layers
                    .cutout
                    .iter()
                    .find(|section| section.section_y == section_y)
                {
                    if let Some(resident) = self.create_resident_terrain_mesh(
                        &section.mesh,
                        Some(owner),
                        Some(mesh_camera_position),
                    ) {
                        self.terrain_cutout.push(resident);
                        self.counters.terrain_uploaded_owners += 1;
                    }
                }
                if let Some(section) = layers
                    .translucent
                    .iter()
                    .find(|section| section.section_y == section_y)
                {
                    if let Some(resident) = self.create_resident_terrain_mesh(
                        &section.mesh,
                        Some(owner),
                        Some(mesh_camera_position),
                    ) {
                        self.terrain_translucent.push(resident);
                        self.counters.terrain_uploaded_owners += 1;
                    }
                }
                self.terrain_section_revisions.insert(owner, revision);
            }
        }

        self.rebuild_translucent_section_indices();
        self.rebuild_translucent_section_order();
        self.resort_translucent_terrain_for_camera();
        self.update_camera();
        self.refresh_terrain_counters();
    }

    /// Applies Vanilla-style recently-compiled section deltas without walking
    /// every resident section. Full synchronization remains the owner for
    /// level/view-area replacement; ordinary compile completion updates only
    /// the stable section owners named here.
    pub fn update_terrain_section_mesh_layer_refs<'a>(
        &mut self,
        source_sections: usize,
        changes: impl IntoIterator<
            Item = (
                TerrainSectionMeshOwner,
                u64,
                &'a terrain::TerrainSectionMeshLayers,
                [f32; 3],
            ),
        >,
        removed: impl IntoIterator<Item = TerrainSectionMeshOwner>,
    ) {
        self.terrain_source_sections = source_sections;
        self.counters.queued_sections = source_sections;
        self.counters.meshed_sections = source_sections;
        self.counters.upload_bytes = 0;
        self.counters.terrain_changed_owners = 0;
        self.counters.terrain_uploaded_owners = 0;
        self.counters.terrain_removed_owners = 0;
        self.counters.terrain_vertex_upload_bytes = 0;
        self.counters.terrain_index_upload_bytes = 0;

        let mut translucent_changed = false;
        for owner in removed {
            self.terrain_section_revisions.remove(&owner);
            self.terrain_section_visibility.remove(&owner);
            self.terrain_visible_section_order
                .retain(|candidate| *candidate != owner);
            self.terrain_nearby_visible_section_order
                .retain(|candidate| *candidate != owner);
            self.counters.terrain_removed_owners +=
                remove_resident_terrain_section(&mut self.terrain_opaque, owner);
            self.counters.terrain_removed_owners +=
                remove_resident_terrain_section(&mut self.terrain_cutout, owner);
            let removed_translucent =
                remove_resident_terrain_section(&mut self.terrain_translucent, owner);
            self.counters.terrain_removed_owners += removed_translucent;
            translucent_changed |= removed_translucent != 0;
        }

        for (owner, revision, layers, mesh_camera_position) in changes {
            if self.terrain_section_revisions.get(&owner) == Some(&revision) {
                continue;
            }
            self.counters.terrain_changed_owners += 1;
            self.counters.terrain_removed_owners +=
                remove_resident_terrain_section(&mut self.terrain_opaque, owner);
            self.counters.terrain_removed_owners +=
                remove_resident_terrain_section(&mut self.terrain_cutout, owner);
            let removed_translucent =
                remove_resident_terrain_section(&mut self.terrain_translucent, owner);
            self.counters.terrain_removed_owners += removed_translucent;
            translucent_changed |= removed_translucent != 0;

            if let Some(section) = layers
                .opaque
                .iter()
                .find(|section| section.section_y == owner.section_y)
            {
                if let Some(resident) = self.create_resident_terrain_mesh(
                    &section.mesh,
                    Some(owner),
                    Some(mesh_camera_position),
                ) {
                    self.terrain_opaque.push(resident);
                    self.counters.terrain_uploaded_owners += 1;
                }
            }
            if let Some(section) = layers
                .cutout
                .iter()
                .find(|section| section.section_y == owner.section_y)
            {
                if let Some(resident) = self.create_resident_terrain_mesh(
                    &section.mesh,
                    Some(owner),
                    Some(mesh_camera_position),
                ) {
                    self.terrain_cutout.push(resident);
                    self.counters.terrain_uploaded_owners += 1;
                }
            }
            if let Some(section) = layers
                .translucent
                .iter()
                .find(|section| section.section_y == owner.section_y)
            {
                if let Some(resident) = self.create_resident_terrain_mesh(
                    &section.mesh,
                    Some(owner),
                    Some(mesh_camera_position),
                ) {
                    self.terrain_translucent.push(resident);
                    self.counters.terrain_uploaded_owners += 1;
                    translucent_changed = true;
                }
            }
            self.terrain_section_revisions.insert(owner, revision);
        }

        if translucent_changed {
            self.rebuild_translucent_section_indices();
            self.rebuild_translucent_section_order();
            self.resort_translucent_terrain_for_camera();
        }
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

    /// Vanilla `Options.maxAnisotropyBit` invalidates the chunk-layer sampler
    /// via `LevelRenderer::resetSampler`; bbb options are startup-only, so the
    /// bit must be set before the real atlas upload recreates the sampler.
    pub fn set_terrain_max_anisotropy_bit(&mut self, max_anisotropy_bit: u32) {
        self.terrain_max_anisotropy_bit = max_anisotropy_bit;
    }

    pub fn upload_terrain_texture_atlas_mips(
        &mut self,
        width: u32,
        height: u32,
        mip_rgba: &[&[u8]],
    ) -> Result<()> {
        let terrain_atlas = create_terrain_atlas_mips_gpu(
            &self.device,
            &self.queue,
            width,
            height,
            mip_rgba,
            effective_terrain_max_anisotropy(
                self.terrain_texture_filtering,
                self.terrain_max_anisotropy_bit,
            ),
        )?;
        let terrain_bind_group = create_terrain_bind_group(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            &terrain_atlas,
        );
        let hud3d_terrain_bind_group = create_terrain_bind_group(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.hud3d_camera_buffer,
            &terrain_atlas,
        );
        let hud_terrain_atlas_bind_group = {
            let (view, sampler) = terrain_atlas.view_and_sampler();
            crate::hud::create_hud_atlas_bind_group(
                &self.device,
                &self.hud_bind_group_layout,
                "bbb-hud-terrain-atlas-bind-group",
                view,
                sampler,
            )
        };
        let gui_item_bind_group = create_terrain_bind_group(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.gui_item_camera_buffer,
            &terrain_atlas,
        );
        if self.resource_reload_in_progress() {
            let previous = std::mem::replace(&mut self.terrain_atlas, terrain_atlas);
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.terrain_atlas = previous;
            }));
            let previous = std::mem::replace(&mut self.terrain_bind_group, terrain_bind_group);
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.terrain_bind_group = previous;
            }));
            let previous =
                std::mem::replace(&mut self.hud3d_terrain_bind_group, hud3d_terrain_bind_group);
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.hud3d_terrain_bind_group = previous;
            }));
            let previous = std::mem::replace(
                &mut self.hud_terrain_atlas_bind_group,
                hud_terrain_atlas_bind_group,
            );
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.hud_terrain_atlas_bind_group = previous;
            }));
            let previous = std::mem::replace(&mut self.gui_item_bind_group, gui_item_bind_group);
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.gui_item_bind_group = previous;
            }));
        } else {
            self.terrain_atlas = terrain_atlas;
            self.terrain_bind_group = terrain_bind_group;
            self.hud3d_terrain_bind_group = hud3d_terrain_bind_group;
            self.hud_terrain_atlas_bind_group = hud_terrain_atlas_bind_group;
            self.gui_item_bind_group = gui_item_bind_group;
        }
        self.counters.atlas_pages = 1;
        self.counters.atlas_reallocations += 1;
        self.counters.atlas_width = width;
        self.counters.atlas_height = height;
        self.invalidate_item_model_atlas(crate::item_models::ItemModelAtlasKind::Blocks);
        Ok(())
    }

    pub fn update_terrain_texture_atlas(&mut self, rgba: &[u8]) -> Result<()> {
        write_terrain_atlas_gpu(&self.queue, &self.terrain_atlas, rgba)
    }

    pub fn update_terrain_texture_atlas_mips(&mut self, mip_rgba: &[&[u8]]) -> Result<()> {
        write_terrain_atlas_mips_gpu(&self.queue, &self.terrain_atlas, mip_rgba)
    }

    pub fn update_terrain_texture_atlas_mip_patches(
        &mut self,
        patches: &[bbb_render_types::AtlasMipPatch],
    ) -> Result<()> {
        write_terrain_atlas_mip_patches_gpu(&self.queue, &self.terrain_atlas, patches)
    }

    pub fn upload_terrain_texture_animation(
        &mut self,
        plan: Option<bbb_render_types::AtlasGpuAnimationPlan>,
    ) -> Result<()> {
        let animation = plan
            .map(|plan| TerrainAtlasAnimationGpu::new(&self.device, &self.queue, plan))
            .transpose()?;
        if animation
            .as_ref()
            .is_some_and(|animation| !animation.matches_atlas(&self.terrain_atlas))
        {
            anyhow::bail!("terrain texture animation plan does not match the resident atlas");
        }
        if self.resource_reload_in_progress() {
            let previous = std::mem::replace(&mut self.terrain_atlas_animation, animation);
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.terrain_atlas_animation = previous;
            }));
        } else {
            self.terrain_atlas_animation = animation;
        }
        Ok(())
    }

    pub fn advance_terrain_texture_animation(&mut self, previous_tick: u64, tick: u64) {
        if let Some(animation) = self.terrain_atlas_animation.as_mut() {
            animation.advance(&self.queue, previous_tick, tick);
        }
    }

    pub(super) fn surface_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.config.width.max(1), self.config.height.max(1))
    }

    /// Public physical-framebuffer query for native callers that need the
    /// world-side surface (camera aspect, culling, screenshots) next to the
    /// logical GUI surface.
    pub fn physical_surface_size(&self) -> PhysicalSize<u32> {
        self.surface_size()
    }

    /// The physical framebuffer size stays `surface_size()`; this is the
    /// narrow logical-GUI split `Minecraft.resizeGui` publishes so HUD layout
    /// and hit testing never read the physical surface. An unchanged size is a
    /// no-op; a change refreshes every camera/uniform state that reads the
    /// logical surface (the GUI-item ortho buffer included) through
    /// `update_camera`, so a later unrelated camera/fog setter is never
    /// required to publish the new projection.
    pub fn set_gui_surface_size(&mut self, size: PhysicalSize<u32>) {
        if self.gui_surface_size == size {
            return;
        }
        self.gui_surface_size = size;
        self.update_camera();
    }

    pub fn gui_surface_size(&self) -> PhysicalSize<u32> {
        self.gui_surface_size
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
        self.counters.visible_sections = if self.terrain_section_revisions.is_empty() {
            self.counters.uploaded_sections
        } else {
            self.terrain_section_visibility
                .keys()
                .filter(|owner| self.terrain_section_revisions.contains_key(owner))
                .count()
        };
        self.counters.resident_bytes = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.resident_bytes)
            .sum();
        self.counters.terrain_peak_resident_bytes = self
            .counters
            .terrain_peak_resident_bytes
            .max(self.counters.resident_bytes);
        self.terrain_bounds = None;
        for bounds in self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .filter_map(|mesh| mesh.bounds)
        {
            if let Some(existing) = &mut self.terrain_bounds {
                existing.include_bounds(bounds);
            } else {
                self.terrain_bounds = Some(bounds);
            }
        }
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
        mesh: &terrain::TerrainMesh,
        owner: Option<TerrainSectionMeshOwner>,
        translucent_camera_position: Option<[f32; 3]>,
    ) -> Option<ResidentTerrainMesh> {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return None;
        }

        let bounds = TerrainBounds::from_vertices(&mesh.vertices);
        let last_point_of_view = owner
            .zip(translucent_camera_position)
            .map(|(owner, camera_position)| translucency_point_of_view(camera_position, owner));
        let translucent_sort = (mesh.translucent_faces > 0)
            .then(|| {
                TerrainTranslucentSortState::from_vertices(
                    &mesh.vertices,
                    &mesh.indices,
                    last_point_of_view,
                )
            })
            .flatten();
        let gpu_vertices = mesh
            .vertices
            .iter()
            .copied()
            .map(super::gpu::TerrainGpuVertex::from)
            .collect::<Vec<_>>();
        let vertex_bytes = bytemuck::cast_slice(&gpu_vertices);
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
        let section_visibility = owner
            .and_then(|owner| self.terrain_section_visibility.get(&owner).copied())
            .unwrap_or(1.0);
        let (section_uniform_buffer, section_bind_group) = create_terrain_section_uniform(
            &self.device,
            &self.terrain_section_bind_group_layout,
            section_visibility,
        );
        let resident_bytes = (vertex_bytes.len() + index_bytes.len() + 16) as u64;
        self.counters.upload_bytes += resident_bytes;
        self.counters.terrain_vertex_upload_bytes = self
            .counters
            .terrain_vertex_upload_bytes
            .saturating_add(vertex_bytes.len() as u64);
        self.counters.terrain_index_upload_bytes = self
            .counters
            .terrain_index_upload_bytes
            .saturating_add(index_bytes.len() as u64);
        Some(ResidentTerrainMesh {
            owner,
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len(),
            index_count: mesh.indices.len(),
            opaque_faces: mesh.opaque_faces,
            cutout_faces: mesh.cutout_faces,
            translucent_faces: mesh.translucent_faces,
            culled_faces: mesh.culled_faces,
            resident_bytes,
            section_visibility,
            _section_uniform_buffer: section_uniform_buffer,
            section_bind_group,
            bounds,
            translucent_sort,
        })
    }

    pub(super) fn terrain_mesh_is_visible(&self, mesh: &ResidentTerrainMesh) -> bool {
        mesh.owner
            .is_none_or(|owner| self.terrain_section_visibility.contains_key(&owner))
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
    /// translucent terrain layer. A valid visible-section traversal is the
    /// authoritative Vanilla order. Distance ordering only applies while
    /// native visibility is unavailable and is not performance-comparison
    /// evidence.
    fn rebuild_translucent_section_order(&mut self) {
        self.terrain_translucent_order = if !self.terrain_section_revisions.is_empty() {
            let owners: Vec<_> = self
                .terrain_translucent
                .iter()
                .map(|mesh| mesh.owner)
                .collect();
            translucent_section_draw_order_from_visible(
                &owners,
                &self.terrain_visible_section_order,
            )
        } else if let Some(camera_position) = self.camera_sort_position() {
            let centers: Vec<Option<[f32; 3]>> = self
                .terrain_translucent
                .iter()
                .map(|mesh| mesh.bounds.map(|bounds| bounds.center().to_array()))
                .collect();
            translucent_section_draw_order(&centers, camera_position)
        } else {
            (0..self.terrain_translucent.len()).collect()
        };
    }

    fn rebuild_translucent_section_indices(&mut self) {
        self.terrain_translucent_indices.clear();
        self.terrain_translucent_indices
            .reserve(self.terrain_translucent.len());
        for (index, mesh) in self.terrain_translucent.iter().enumerate() {
            if let Some(owner) = mesh.owner {
                self.terrain_translucent_indices.insert(owner, index);
            }
        }
    }

    fn resort_translucent_terrain_for_camera(&mut self) {
        // A committed native visibility traversal already owns Vanilla's
        // section draw order and does not change with sub-frame camera motion.
        // Only the pre-visibility startup path is distance-sorted here;
        // mesh/visibility mutation paths rebuild the authoritative order at the
        // mutation site.
        if self.terrain_section_revisions.is_empty() {
            self.rebuild_translucent_section_order();
        }
        self.counters.terrain_translucent_resort_owners = 0;
        self.counters.terrain_translucent_resort_index_bytes = 0;

        let Some(camera_position) = self.camera_sort_position() else {
            return;
        };
        let camera_block = camera_block_position(camera_position);
        let block_position_changed =
            self.last_translucent_sort_block_position != Some(camera_block);

        if !self.terrain_section_revisions.is_empty() {
            // Vanilla 26.1 schedules direct references from the nearby list,
            // then direct references from a round-robin visible-section
            // window. Native carries both lists from the same octree visitor;
            // use the stable resident index instead of rebuilding a temporary
            // owner set and rescanning every translucent mesh.
            let visible = &self.terrain_visible_section_order;
            if visible.is_empty() {
                return;
            }
            let meshes = &mut self.terrain_translucent;
            let resident_indices = &self.terrain_translucent_indices;
            for &owner in &self.terrain_nearby_visible_section_order {
                schedule_translucent_section(
                    meshes,
                    resident_indices,
                    owner,
                    camera_position,
                    block_position_changed,
                    true,
                );
            }
            self.translucency_resort_iteration_index %= visible.len();
            let resorts = (visible.len() / 8).max(15);
            for _ in 0..resorts {
                let owner = visible[self.translucency_resort_iteration_index % visible.len()];
                self.translucency_resort_iteration_index =
                    self.translucency_resort_iteration_index.saturating_add(1);
                schedule_translucent_section(
                    meshes,
                    resident_indices,
                    owner,
                    camera_position,
                    block_position_changed,
                    false,
                );
            }
        } else {
            for mesh in &mut self.terrain_translucent {
                let (point_of_view, nearby) = match mesh.owner {
                    Some(owner) => (
                        translucency_point_of_view(camera_position, owner),
                        translucent_section_is_nearby(camera_position, owner),
                    ),
                    None => ([0, 0, 0], true),
                };
                let Some(sort) = &mut mesh.translucent_sort else {
                    continue;
                };
                sort.schedule_for_camera_if_needed(
                    camera_position,
                    point_of_view,
                    block_position_changed,
                    nearby,
                );
            }
        }

        let mesh_count = self.terrain_translucent.len();
        let mut visited = 0;
        let mut completed = 0;
        while visited < mesh_count && completed < MAX_TRANSLUCENT_RESORTS_PER_CAMERA_UPDATE {
            let index = self.translucency_resort_drain_index % mesh_count;
            self.translucency_resort_drain_index =
                self.translucency_resort_drain_index.saturating_add(1);
            visited += 1;
            let mesh = &mut self.terrain_translucent[index];
            let Some(indices) = mesh
                .translucent_sort
                .as_mut()
                .and_then(TerrainTranslucentSortState::take_scheduled_indices)
            else {
                continue;
            };
            let index_bytes = bytemuck::cast_slice(&indices);
            self.queue.write_buffer(&mesh.index_buffer, 0, index_bytes);
            self.counters.terrain_translucent_resort_owners += 1;
            self.counters.terrain_translucent_resort_index_bytes = self
                .counters
                .terrain_translucent_resort_index_bytes
                .saturating_add(index_bytes.len() as u64);
            completed += 1;
        }
        self.last_translucent_sort_block_position = Some(camera_block);
    }

    pub(crate) fn update_camera(&self) {
        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        let glint_elapsed_millis = self.started_at.elapsed().as_secs_f64() * 1000.0;
        let uniform = if let Some(pose) = self.camera_pose {
            CameraUniform::from_pose_with_camera_effects(
                pose,
                aspect,
                self.camera_fov_degrees,
                self.camera_hurt_effect,
                self.camera_view_bob_effect,
                self.camera_spinning_effect,
            )
        } else {
            self.scene_bounds()
                .map(|bounds| CameraUniform::from_bounds(bounds, aspect))
                .unwrap_or_else(CameraUniform::identity)
        }
        .with_lightmap_environment(self.lightmap_environment)
        .with_fog_environment(self.fog_environment)
        .with_glint_texture_time(glint_elapsed_millis, self.glint_speed, self.glint_alpha)
        .with_shader_game_time_ticks(self.shader_game_time_ticks)
        .with_viewport_size(self.config.width as f32, self.config.height as f32);
        self.update_sky_model_view_dynamics(uniform.sky_model_view_transform());
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
        let hud3d = if let Some(pose) = self.camera_pose {
            CameraUniform::from_hud_pose_with_view_bob(
                pose,
                aspect,
                self.camera_hud_fov_degrees,
                self.camera_hurt_effect,
                self.camera_view_bob_effect,
            )
        } else {
            self.scene_bounds()
                .map(|bounds| CameraUniform::from_bounds(bounds, aspect))
                .unwrap_or_else(CameraUniform::identity)
        }
        .with_lightmap_environment(self.lightmap_environment)
        .with_fog_environment(self.fog_environment)
        .with_glint_texture_time(glint_elapsed_millis, self.glint_speed, self.glint_alpha)
        .with_shader_game_time_ticks(self.shader_game_time_ticks)
        .with_viewport_size(self.config.width as f32, self.config.height as f32);
        self.queue
            .write_buffer(&self.hud3d_camera_buffer, 0, bytemuck::bytes_of(&hud3d));
        // The GUI item pass projects 3D inventory icons with a screen-space
        // ortho over the logical GUI surface (`Window.setGuiScale`), so a
        // 1920x1080 framebuffer at scale 4 lays icons out in 480x270 exactly
        // like Vanilla. The separate buffer does not clobber the world camera,
        // which earlier passes in the same submit still read.
        let gui = CameraUniform::gui_ortho(
            self.gui_surface_size.width as f32,
            self.gui_surface_size.height as f32,
        )
        .with_glint_texture_time(glint_elapsed_millis, self.glint_speed, self.glint_alpha)
        .with_shader_game_time_ticks(self.shader_game_time_ticks)
        .with_viewport_size(self.config.width as f32, self.config.height as f32);
        self.queue
            .write_buffer(&self.gui_item_camera_buffer, 0, bytemuck::bytes_of(&gui));
        #[cfg(test)]
        self.camera_uniform_writes
            .set(self.camera_uniform_writes.get().saturating_add(1));
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
        let mut environment = self.lightmap_environment;
        environment.brightness_factor = sanitize_lightmap_brightness_factor(factor);
        self.set_lightmap_environment(environment);
    }

    /// Vanilla `GameRenderer.extractRenderState` samples `Options.glintSpeed`
    /// into the glint texture transform and `Options.glintStrength` into the
    /// `GlobalSettingsUniform` glint alpha every frame; the startup-loaded
    /// option values arrive here once. Non-finite or negative inputs keep the
    /// vanilla defaults.
    pub fn set_glint_settings(&mut self, speed: f64, strength: f32) {
        if speed.is_finite() && speed >= 0.0 {
            self.glint_speed = speed;
        }
        if strength.is_finite() && strength >= 0.0 {
            self.glint_alpha = strength;
        }
    }

    pub fn set_lightmap_block_factor(&mut self, factor: f32) {
        let mut environment = self.lightmap_environment;
        environment.block_factor = sanitize_lightmap_block_factor(factor);
        self.set_lightmap_environment(environment);
    }

    pub fn set_lightmap_environment(&mut self, environment: LightmapEnvironment) {
        if !self.replace_lightmap_environment(environment) {
            return;
        }
        self.update_camera();
    }

    fn replace_lightmap_environment(&mut self, environment: LightmapEnvironment) -> bool {
        let environment = environment.sanitized();
        if self.lightmap_environment == environment {
            return false;
        }
        self.lightmap_environment = environment;
        // Vanilla 26.1 `LightmapRenderStateExtractor.tick` dirties the
        // lightmap once per client tick, and `Lightmap.render` consumes that
        // state once. The native simulation publication carries that complete
        // tick-owned projection, so repeated presentation of the same
        // publication must keep sampling the resident 16x16 texture rather
        // than redrawing it every frame.
        self.lightmap_dirty = true;
        true
    }

    pub fn set_fog_environment(&mut self, environment: FogEnvironment) {
        self.replace_fog_environment(environment);
        self.update_camera();
    }

    fn replace_fog_environment(&mut self, environment: FogEnvironment) {
        self.fog_environment = environment.sanitized();
    }

    pub fn set_sky_environment(&mut self, environment: SkyEnvironment) {
        if !self.replace_sky_environment(environment) {
            return;
        }
        self.update_camera();
    }

    fn replace_sky_environment(&mut self, environment: SkyEnvironment) -> bool {
        let environment = environment.sanitized();
        if self.sky_environment == environment {
            return false;
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
        true
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

    /// Vanilla `Options.cloudStatus`: `OFF` skips the cloud pass, `FAST`
    /// builds the flat mesh, `FANCY` the boxed one.
    pub fn set_cloud_status(&mut self, enabled: bool, shape: CloudShape) {
        self.clouds_enabled = enabled;
        self.set_cloud_shape(shape);
    }

    pub fn set_cloud_shape(&mut self, shape: CloudShape) {
        if self.cloud_shape == shape {
            return;
        }
        self.cloud_shape = shape;
        self.rebuild_clouds();
    }

    pub fn upload_end_sky_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.upload_end_sky_texture_with_sampling(
            width,
            height,
            rgba,
            StandaloneTextureSampling::default(),
        )
    }

    pub fn upload_end_sky_texture_with_sampling(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        self.end_sky_texture = Some(create_end_sky_texture_gpu(
            &self.device,
            &self.queue,
            &self.end_sky_texture_bind_group_layout,
            width,
            height,
            rgba,
            sampling,
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
        self.upload_world_border_texture_with_sampling(
            width,
            height,
            rgba,
            StandaloneTextureSampling::default(),
        )
    }

    pub fn upload_world_border_texture_with_sampling(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        self.world_border_texture = Some(create_world_border_texture_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            width,
            height,
            rgba,
            sampling,
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
            self.cloud_environment,
            self.cloud_frame,
            self.cloud_texture.as_ref(),
        );
    }

    pub(super) fn restore_cloud_uniform_after_resource_reload(&self) {
        self.write_cloud_uniform();
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
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
            )
        })
        .ok_or_else(|| {
            anyhow!("surface does not expose a Vanilla-compatible RGBA/BGRA 8-bit UNORM format")
        })
}

fn sanitize_camera_fov_degrees(fov_degrees: f32) -> f32 {
    if fov_degrees.is_finite() {
        fov_degrees.clamp(0.1, 179.0)
    } else {
        70.0
    }
}

fn sanitize_shader_game_time_ticks(ticks: f64) -> f64 {
    if ticks.is_finite() {
        ticks.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        choose_format, choose_present_mode, translucency_point_of_view,
        translucent_section_draw_order, translucent_section_draw_order_from_visible,
        translucent_section_is_nearby, LevelFrameRoute, Renderer, RendererCameraFrame,
        RendererResizePrepareStage, TerrainChunkMeshOwner, TerrainSectionMeshOwner,
        TerrainSectionRenderState, TerrainTranslucentSortState,
        MAX_TRANSLUCENT_RESORTS_PER_CAMERA_UPDATE,
    };
    use crate::camera::{CameraPose, LightmapEnvironment, TerrainBounds};
    use crate::terrain::{
        TerrainMesh, TerrainSectionMesh, TerrainSectionMeshLayers, TerrainVertex,
    };
    use glam::Vec3;
    use winit::dpi::PhysicalSize;

    fn resize_target_sizes(renderer: &Renderer) -> Vec<wgpu::Extent3d> {
        vec![
            renderer.screen_blur_target.texture.size(),
            renderer.main_target.texture.size(),
            renderer.depth._texture.size(),
            renderer.translucent_target._texture.size(),
            renderer.item_entity_target._texture.size(),
            renderer.particle_target._texture.size(),
            renderer.weather_target._texture.size(),
            renderer.transparency_final_target._texture.size(),
            renderer.entity_outline_target._texture.size(),
            renderer.cloud_target._texture.size(),
        ]
    }

    #[test]
    fn level_frame_route_matches_vanilla_option_and_panoramic_suppression() {
        assert_eq!(LevelFrameRoute::default(), LevelFrameRoute::Regular);
        assert_eq!(
            LevelFrameRoute::for_options(false, false),
            LevelFrameRoute::Regular
        );
        assert_eq!(
            LevelFrameRoute::for_options(true, false),
            LevelFrameRoute::ImprovedTransparency
        );
        assert_eq!(
            LevelFrameRoute::for_options(true, true),
            LevelFrameRoute::Regular
        );
    }

    #[test]
    fn resize_prepare_failures_keep_the_previous_complete_backend_generation() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 12) else {
            return;
        };
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_post_effect(Some("minecraft:invert"), true);
        let previous = renderer.render_offscreen_frame().unwrap();
        let previous_pixel = previous.pixel(2, 2);
        let previous_sizes = resize_target_sizes(&renderer);
        let previous_surface_size = renderer.surface_size();

        for rejected in [
            RendererResizePrepareStage::PostEffects,
            RendererResizePrepareStage::FrameTargets,
        ] {
            let counters = renderer.counters();
            let error = renderer
                .resize_with_prepare_check(PhysicalSize::new(24, 20), |stage| {
                    if stage == rejected {
                        anyhow::bail!("injected {stage:?} resize prepare failure");
                    }
                    Ok(())
                })
                .unwrap_err();
            assert!(error.to_string().contains("injected"));
            assert_eq!(renderer.surface_size(), previous_surface_size);
            assert_eq!(resize_target_sizes(&renderer), previous_sizes);
            assert_eq!(renderer.counters(), counters);
            assert_eq!(renderer.current_post_effect_id(), Some("minecraft:invert"));

            let retained = renderer.render_offscreen_frame().unwrap();
            assert_eq!((retained.width, retained.height), (16, 12));
            assert_eq!(retained.pixel(2, 2), previous_pixel);
        }

        let previous_resizes = renderer.counters().surface_resizes;
        renderer.resize(PhysicalSize::new(24, 20)).unwrap();
        assert_eq!(renderer.surface_size(), PhysicalSize::new(24, 20));
        assert!(resize_target_sizes(&renderer).iter().all(|size| {
            size.width == 24 && size.height == 20 && size.depth_or_array_layers == 1
        }));
        assert_eq!(renderer.counters().surface_resizes, previous_resizes + 1);
        let resized = renderer.render_offscreen_frame().unwrap();
        assert_eq!((resized.width, resized.height), (24, 20));
        assert_eq!(resized.pixel(2, 2), previous_pixel);
    }

    #[test]
    fn zero_size_resize_skips_prepare_and_preserves_backend_owners() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 12) else {
            return;
        };
        let previous_sizes = resize_target_sizes(&renderer);
        let previous_surface_size = renderer.surface_size();
        let previous_resizes = renderer.counters().surface_resizes;
        let previous_skips = renderer.counters().surface_zero_size_skips;
        let mut prepared = false;

        renderer
            .resize_with_prepare_check(PhysicalSize::new(0, 12), |_| {
                prepared = true;
                Ok(())
            })
            .unwrap();

        assert!(!prepared);
        assert_eq!(renderer.surface_size(), previous_surface_size);
        assert_eq!(resize_target_sizes(&renderer), previous_sizes);
        assert_eq!(renderer.counters().surface_resizes, previous_resizes);
        assert_eq!(
            renderer.counters().surface_zero_size_skips,
            previous_skips + 1
        );
    }

    #[test]
    fn set_vsync_enabled_renegotiates_present_mode_without_touching_targets() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 12) else {
            return;
        };
        let previous_sizes = resize_target_sizes(&renderer);
        assert_eq!(renderer.config.present_mode, wgpu::PresentMode::Fifo);

        renderer.present_modes = vec![wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate];
        renderer.set_vsync_enabled(false).unwrap();
        assert_eq!(renderer.config.present_mode, wgpu::PresentMode::Immediate);
        assert!(!renderer.vsync_enabled);
        assert_eq!(renderer.surface_size(), PhysicalSize::new(16, 12));
        assert_eq!(resize_target_sizes(&renderer), previous_sizes);

        renderer.set_vsync_enabled(true).unwrap();
        assert_eq!(renderer.config.present_mode, wgpu::PresentMode::Fifo);
        assert!(renderer.vsync_enabled);
    }

    #[test]
    fn set_vsync_enabled_survives_zero_size_without_reconfigure() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 12) else {
            return;
        };
        renderer.present_modes = vec![wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate];
        renderer.resize(PhysicalSize::new(0, 0)).unwrap();
        renderer.set_vsync_enabled(false).unwrap();
        assert_eq!(renderer.config.present_mode, wgpu::PresentMode::Immediate);
        assert_eq!(renderer.surface_size(), PhysicalSize::new(16, 12));

        renderer.resize(PhysicalSize::new(24, 20)).unwrap();
        assert_eq!(renderer.config.present_mode, wgpu::PresentMode::Immediate);
        assert_eq!(renderer.surface_size(), PhysicalSize::new(24, 20));
    }

    #[test]
    fn gui_surface_size_stays_logical_while_surface_size_stays_physical() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(1920, 1080) else {
            return;
        };
        // Defaults to the physical surface until the first resizeGui
        // publication (scale 1).
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(1920, 1080));
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1920, 1080));

        // 1920x1080 at auto scale 4 publishes 480x270 to the renderer.
        renderer.set_gui_surface_size(PhysicalSize::new(480, 270));
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1920, 1080));

        // A zero-size resize only marks minimized; both last usable sizes are
        // retained until a nonzero resize replaces the physical surface.
        renderer.resize(PhysicalSize::new(0, 0)).unwrap();
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1920, 1080));
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));

        renderer.resize(PhysicalSize::new(1280, 720)).unwrap();
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1280, 720));
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));
    }

    #[test]
    fn set_gui_surface_size_refreshes_dependent_camera_state_and_no_ops_when_unchanged() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(1920, 1080) else {
            return;
        };
        let baseline = renderer.camera_uniform_writes.get();

        // A changed logical GUI size publishes through `update_camera`, which
        // rewrites the GUI-item ortho buffer from the new size; the world
        // camera keeps reading the physical config.
        renderer.set_gui_surface_size(PhysicalSize::new(480, 270));
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));
        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 1);
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1920, 1080));

        // An unchanged size is a true no-op, not a redundant uniform rewrite.
        renderer.set_gui_surface_size(PhysicalSize::new(480, 270));
        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 1);

        // A second change refreshes again; 1280x720 at scale 3 projects
        // 427x240 (ceiling division).
        renderer.set_gui_surface_size(PhysicalSize::new(427, 240));
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(427, 240));
        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 2);
    }

    #[test]
    fn camera_frame_publishes_all_uniform_inputs_once() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(1920, 1080) else {
            return;
        };
        let baseline = renderer.camera_uniform_writes.get();
        let pose = CameraPose {
            position: [8.0, 72.0, 8.0],
            y_rot: 45.0,
            x_rot: -12.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let mut lightmap_environment = LightmapEnvironment::default();
        lightmap_environment.sky_factor = 0.5;

        renderer.set_camera_frame(RendererCameraFrame {
            lightmap_environment,
            fog_environment: crate::FogEnvironment::world([0.1, 0.2, 0.3, 1.0], 0.0, 96.0, 12),
            sky_environment: crate::SkyEnvironment::from_rgb([0.2, 0.3, 0.4]),
            pose: Some(pose),
            hurt_effect: Some(crate::CameraHurtEffect {
                hurt_time: 4.0,
                hurt_duration: 10.0,
                hurt_dir: 15.0,
                death_time: 0.0,
                is_dead_or_dying: false,
                damage_tilt_strength: 1.0,
            }),
            view_bob_effect: Some(crate::CameraViewBobEffect {
                backwards_interpolated_walk_distance: 2.0,
                bob: 0.25,
            }),
            spinning_effect: Some(crate::CameraSpinningEffect {
                intensity: 0.2,
                angle_degrees: 30.0,
            }),
            fov_degrees: 90.0,
            hud_fov_degrees: 80.0,
            shader_game_time_ticks: 6000.5,
            ..RendererCameraFrame::default()
        });

        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 1);
        assert_eq!(renderer.camera_pose, Some(pose));
        assert_eq!(renderer.lightmap_environment, lightmap_environment);
        assert_eq!(renderer.camera_fov_degrees, 90.0);
        assert_eq!(renderer.camera_hud_fov_degrees, 80.0);
        assert_eq!(renderer.shader_game_time_ticks, 6000.5);
        assert!(renderer.lightmap_dirty);

        // The frame boundary still refreshes real-time glint state while the
        // simulation is paused, but never more than once per publication.
        renderer.set_camera_frame(RendererCameraFrame {
            lightmap_environment,
            fog_environment: renderer.fog_environment,
            sky_environment: renderer.sky_environment,
            pose: Some(pose),
            hurt_effect: renderer.camera_hurt_effect,
            view_bob_effect: renderer.camera_view_bob_effect,
            spinning_effect: renderer.camera_spinning_effect,
            fov_degrees: 90.0,
            hud_fov_degrees: 80.0,
            shader_game_time_ticks: 6000.5,
            ..RendererCameraFrame::default()
        });
        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 2);
    }

    #[test]
    fn gui_item_camera_uses_the_logical_surface_and_world_camera_stays_physical() {
        let source = include_str!("renderer.rs");
        let production = source
            .split_once("\n#[cfg(test)]\nmod tests")
            .expect("renderer production/test boundary")
            .0;
        let update_camera = production
            .split_once("pub(crate) fn update_camera(&self) {")
            .expect("camera owner")
            .1;
        // The camera owner is `&self`; the call counter is a `#[cfg(test)]`
        // seam and never mutates production state for a test.
        assert!(update_camera.contains("#[cfg(test)]"));
        // World camera aspect/viewport stays on the physical config.
        assert!(update_camera
            .contains("let aspect = self.config.width as f32 / self.config.height.max(1) as f32;"));
        // The GUI item ortho covers the logical GUI surface.
        assert!(update_camera.contains("CameraUniform::gui_ortho("));
        assert!(update_camera.contains("self.gui_surface_size.width as f32"));
        assert!(update_camera.contains("self.gui_surface_size.height as f32"));
    }

    #[test]
    fn production_resize_has_one_prepare_apply_path() {
        let source = include_str!("renderer.rs");
        let production = source
            .split_once("\n#[cfg(test)]\nmod tests")
            .expect("renderer production/test boundary")
            .0;
        let public_resize = production
            .split_once("    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {")
            .expect("single public resize entry")
            .1;
        assert_eq!(
            production
                .matches("    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {")
                .count(),
            1
        );
        assert!(
            public_resize.starts_with("\n        self.resize_with_prepare_check(size, |_| Ok(()))")
        );
        assert_eq!(production.matches("self.apply_resize(plan);").count(), 1);
        assert!(!production.contains("post_effect_graphs.resize("));
        assert!(!production.contains("camera_post_effect.resize("));
    }

    #[test]
    fn renderer_backend_gpu_owners_share_one_struct_raii_lifetime() {
        let source = include_str!("renderer.rs");
        let production = source
            .split_once("\n#[cfg(test)]\nmod tests")
            .expect("renderer production/test boundary")
            .0;
        assert_eq!(production.matches("pub struct Renderer {").count(), 1);
        assert_eq!(production.matches("Self::with_gpu(").count(), 1);
        assert!(!production.contains("impl Drop for Renderer"));
        assert!(!production.contains("ManuallyDrop<"));
        assert!(!production.contains("std::mem::forget("));
        assert!(!production.contains("fn close_backend"));
    }

    #[test]
    fn choose_format_requires_vanilla_rgba8_numeric_surface_behavior() {
        assert_eq!(
            choose_format(&[
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba16Float,
            ])
            .unwrap(),
            wgpu::TextureFormat::Rgba8Unorm
        );
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
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Depth24Plus,
        ])
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("surface does not expose a Vanilla-compatible RGBA/BGRA 8-bit UNORM format"));
    }

    #[test]
    fn choose_present_mode_tracks_vsync_preference() {
        assert_eq!(
            choose_present_mode(
                &[
                    wgpu::PresentMode::Fifo,
                    wgpu::PresentMode::Immediate,
                    wgpu::PresentMode::Mailbox,
                ],
                true,
            )
            .unwrap(),
            wgpu::PresentMode::Fifo
        );
        assert_eq!(
            choose_present_mode(
                &[
                    wgpu::PresentMode::Fifo,
                    wgpu::PresentMode::Immediate,
                    wgpu::PresentMode::Mailbox,
                ],
                false,
            )
            .unwrap(),
            wgpu::PresentMode::Immediate
        );
        assert_eq!(
            choose_present_mode(
                &[wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox],
                false
            )
            .unwrap(),
            wgpu::PresentMode::Mailbox
        );
        assert_eq!(
            choose_present_mode(&[wgpu::PresentMode::Fifo], false).unwrap(),
            wgpu::PresentMode::Fifo
        );
    }

    #[test]
    fn camera_post_effect_setter_commits_selection_and_active_state_together() {
        let source = include_str!("renderer.rs");
        let setter = source
            .split("pub fn set_camera_post_effect")
            .nth(1)
            .and_then(|source| source.split_once("\n    }"))
            .map(|(body, _)| body)
            .expect("camera post-effect setter");
        assert!(setter.contains("self.current_post_effect_id = id.map(str::to_owned);"));
        assert!(setter.contains("self.camera_post_effect_active = active;"));
    }

    #[test]
    fn lightmap_dirty_state_tracks_tick_owned_environment_changes() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        assert!(
            renderer.lightmap_dirty,
            "the initial resident texture requires one update"
        );

        renderer.lightmap_dirty = false;
        renderer.set_lightmap_environment(LightmapEnvironment::default());
        assert!(
            !renderer.lightmap_dirty,
            "presenting the same simulation publication must reuse the resident lightmap"
        );

        renderer.set_lightmap_brightness_factor(0.75);
        assert!(renderer.lightmap_dirty);
        renderer.lightmap_dirty = false;
        renderer.set_lightmap_brightness_factor(0.75);
        assert!(
            !renderer.lightmap_dirty,
            "replaying an unchanged option value must not redraw the lightmap"
        );

        renderer.set_lightmap_block_factor(1.25);
        assert!(renderer.lightmap_dirty);
    }

    #[test]
    fn borrowed_terrain_upload_keeps_cpu_mesh_cache_available() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let cached = TerrainMesh {
            vertices: vec![
                terrain_vertex([0.0, 0.0, 0.0]),
                terrain_vertex([1.0, 0.0, 0.0]),
                terrain_vertex([0.0, 1.0, 0.0]),
            ],
            indices: vec![0, 1, 2],
            source_sections: 1,
            opaque_faces: 1,
            ..TerrainMesh::default()
        };

        renderer.upload_terrain_mesh_layer_refs(
            1,
            std::iter::once(&cached),
            std::iter::empty(),
            std::iter::empty(),
        );

        assert_eq!(cached.vertices.len(), 3);
        assert_eq!(cached.indices, vec![0, 1, 2]);
        assert_eq!(renderer.counters().queued_sections, 1);
        assert_eq!(renderer.counters().meshed_sections, 1);
        assert_eq!(renderer.counters().uploaded_sections, 1);
        assert_eq!(renderer.counters().terrain_vertices, 3);
        assert!(renderer.counters().upload_bytes > 0);
    }

    #[test]
    fn terrain_residency_observation_crosses_the_renderer_counter_boundary() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_terrain_mesh_worker_counters(crate::TerrainMeshWorkerCounters {
            selected_chunks: 625,
            resident_chunks: 625,
            selected_sections: 15_000,
            resident_sections: 15_000,
            pending_jobs: 0,
            cache_ready: true,
            ..Default::default()
        });

        assert_eq!(renderer.counters().terrain_selected_chunks, 625);
        assert_eq!(renderer.counters().terrain_resident_chunks, 625);
        assert_eq!(renderer.counters().terrain_selected_sections, 15_000);
        assert_eq!(renderer.counters().terrain_resident_sections, 15_000);
        assert_eq!(renderer.counters().terrain_pending_jobs, 0);
        assert!(renderer.counters().terrain_cache_ready);
    }

    #[test]
    fn incremental_terrain_residency_replaces_only_changed_section_owners() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let layers = TerrainSectionMeshLayers {
            opaque: vec![TerrainSectionMesh {
                section_y: 4,
                mesh: TerrainMesh {
                    vertices: vec![
                        terrain_vertex([0.0, 64.0, 0.0]),
                        terrain_vertex([1.0, 64.0, 0.0]),
                        terrain_vertex([0.0, 65.0, 0.0]),
                    ],
                    indices: vec![0, 1, 2],
                    source_sections: 1,
                    opaque_faces: 1,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        let west = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: -1,
            chunk_z: 0,
        };
        let east = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: 0,
            chunk_z: 0,
        };
        renderer.synchronize_terrain_section_mesh_layer_refs(
            2,
            [
                (west, &[(4, 1)][..], &layers, [0.0; 3]),
                (east, &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
        let one_owner_bytes = renderer.counters().upload_bytes / 2;
        assert_eq!(renderer.counters().terrain_changed_owners, 2);
        assert_eq!(renderer.counters().terrain_uploaded_owners, 2);
        assert_eq!(renderer.counters().terrain_full_replacements, 0);
        assert_eq!(renderer.terrain_opaque.len(), 2);

        renderer.synchronize_terrain_section_mesh_layer_refs(
            2,
            [
                (west, &[(4, 1)][..], &layers, [0.0; 3]),
                (east, &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
        assert_eq!(renderer.counters().terrain_changed_owners, 0);
        assert_eq!(renderer.counters().terrain_uploaded_owners, 0);
        assert_eq!(renderer.counters().terrain_removed_owners, 0);
        assert_eq!(renderer.counters().upload_bytes, 0);
        assert_eq!(renderer.terrain_opaque.len(), 2);

        renderer.synchronize_terrain_section_mesh_layer_refs(
            2,
            [
                (west, &[(4, 2)][..], &layers, [0.0; 3]),
                (east, &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
        assert_eq!(renderer.counters().terrain_changed_owners, 1);
        assert_eq!(renderer.counters().terrain_uploaded_owners, 1);
        assert_eq!(renderer.counters().terrain_removed_owners, 1);
        assert_eq!(renderer.counters().upload_bytes, one_owner_bytes);
        assert_eq!(renderer.counters().terrain_full_replacements, 0);
        assert_eq!(renderer.terrain_opaque.len(), 2);

        renderer.synchronize_terrain_section_mesh_layer_refs(
            1,
            [(east, &[(4, 1)][..], &layers, [0.0; 3])],
        );
        assert_eq!(renderer.counters().terrain_changed_owners, 0);
        assert_eq!(renderer.counters().terrain_uploaded_owners, 0);
        assert_eq!(renderer.counters().terrain_removed_owners, 1);
        assert_eq!(renderer.terrain_opaque.len(), 1);
        assert_eq!(
            renderer.terrain_opaque[0]
                .owner
                .expect("incremental residents have owners")
                .chunk,
            east
        );

        let next_level_east = TerrainChunkMeshOwner {
            level_generation: 8,
            ..east
        };
        renderer.synchronize_terrain_section_mesh_layer_refs(
            1,
            [(next_level_east, &[(4, 1)][..], &layers, [0.0; 3])],
        );
        assert_eq!(renderer.counters().terrain_changed_owners, 1);
        assert_eq!(renderer.counters().terrain_uploaded_owners, 1);
        assert_eq!(renderer.counters().terrain_removed_owners, 1);
        assert_eq!(renderer.counters().terrain_full_replacements, 0);
    }

    #[test]
    fn recently_compiled_terrain_delta_does_not_scan_or_replace_sibling_owner() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let layers = TerrainSectionMeshLayers {
            opaque: vec![TerrainSectionMesh {
                section_y: 4,
                mesh: TerrainMesh {
                    vertices: vec![
                        terrain_vertex([0.0, 64.0, 0.0]),
                        terrain_vertex([1.0, 64.0, 0.0]),
                        terrain_vertex([0.0, 65.0, 0.0]),
                    ],
                    indices: vec![0, 1, 2],
                    source_sections: 1,
                    opaque_faces: 1,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        let west = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: -1,
            chunk_z: 0,
        };
        let east = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: 0,
            chunk_z: 0,
        };
        let west_section = TerrainSectionMeshOwner {
            chunk: west,
            section_y: 4,
        };
        let east_section = TerrainSectionMeshOwner {
            chunk: east,
            section_y: 4,
        };
        renderer.synchronize_terrain_section_mesh_layer_refs(
            2,
            [
                (west, &[(4, 1)][..], &layers, [0.0; 3]),
                (east, &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
        assert_eq!(
            renderer.counters().visible_sections,
            0,
            "new residency waits for the native membership/fade projection"
        );
        renderer.set_terrain_section_states(
            1,
            &[TerrainSectionRenderState {
                owner: east_section,
                visibility: 0.4,
                nearby: true,
            }],
        );

        renderer.update_terrain_section_mesh_layer_refs(
            2,
            [(west_section, 2, &layers, [0.0; 3])],
            [],
        );
        assert_eq!(renderer.counters().terrain_changed_owners, 1);
        assert_eq!(renderer.counters().terrain_uploaded_owners, 1);
        assert_eq!(renderer.counters().terrain_removed_owners, 1);
        assert_eq!(renderer.counters().visible_sections, 1);
        assert_eq!(renderer.terrain_opaque.len(), 2);
        assert!(renderer.terrain_opaque.iter().any(|mesh| {
            mesh.owner == Some(east_section)
                && mesh.section_visibility.to_bits() == 0.4f32.to_bits()
        }));
        assert_eq!(
            renderer.terrain_section_revisions.get(&east_section),
            Some(&1)
        );

        renderer.update_terrain_section_mesh_layer_refs(
            2,
            [(east_section, 2, &layers, [0.0; 3])],
            [],
        );
        assert_eq!(renderer.counters().visible_sections, 1);
        assert!(renderer.terrain_opaque.iter().any(|mesh| {
            mesh.owner == Some(east_section)
                && mesh.section_visibility.to_bits() == 0.4f32.to_bits()
        }));

        renderer.update_terrain_section_mesh_layer_refs(1, std::iter::empty(), [east_section]);
        assert_eq!(renderer.counters().terrain_changed_owners, 0);
        assert_eq!(renderer.counters().terrain_removed_owners, 1);
        assert_eq!(renderer.counters().visible_sections, 0);
        assert_eq!(renderer.terrain_opaque.len(), 1);
    }

    #[test]
    fn new_section_residency_cannot_restore_the_one_frame_visibility_bridge() {
        let bridge = ["terrain_section_visibility.insert(owner, ", "1.0)"].concat();
        assert!(!include_str!("renderer.rs").contains(&bridge));
    }

    #[test]
    fn terrain_visibility_filter_limits_resident_section_draws_and_counter() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let layers = TerrainSectionMeshLayers {
            opaque: vec![TerrainSectionMesh {
                section_y: 4,
                mesh: TerrainMesh {
                    vertices: vec![
                        terrain_vertex([0.0, 64.0, 0.0]),
                        terrain_vertex([1.0, 64.0, 0.0]),
                        terrain_vertex([0.0, 65.0, 0.0]),
                    ],
                    indices: vec![0, 1, 2],
                    source_sections: 1,
                    opaque_faces: 1,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        let west = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: -1,
            chunk_z: 0,
        };
        let east = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: 0,
            chunk_z: 0,
        };
        let north = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: 0,
            chunk_z: -1,
        };

        renderer.synchronize_terrain_section_mesh_layer_refs(
            3,
            [
                (west, &[(4, 1)][..], &layers, [0.0; 3]),
                (east, &[(4, 1)][..], &layers, [0.0; 3]),
                (north, &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
        renderer.set_terrain_section_states(
            1,
            &[TerrainSectionRenderState {
                owner: TerrainSectionMeshOwner {
                    chunk: east,
                    section_y: 4,
                },
                visibility: 1.0,
                nearby: true,
            }],
        );
        assert_eq!(renderer.terrain_section_uniform_scan_count, 0);
        renderer.set_terrain_section_states(
            2,
            &[TerrainSectionRenderState {
                owner: TerrainSectionMeshOwner {
                    chunk: west,
                    section_y: 4,
                },
                visibility: 1.0,
                nearby: true,
            }],
        );
        renderer.set_terrain_section_states(
            3,
            &[TerrainSectionRenderState {
                owner: TerrainSectionMeshOwner {
                    chunk: east,
                    section_y: 4,
                },
                visibility: 1.0,
                nearby: true,
            }],
        );
        assert_eq!(
            renderer.terrain_section_uniform_scan_count, 0,
            "fully visible membership changes do not rescan resident GPU meshes"
        );
        assert!(renderer.terrain_next_section_visibility.capacity() >= 1);
        assert!(renderer.terrain_next_visible_section_order.capacity() >= 1);

        assert_eq!(renderer.counters().uploaded_sections, 3);
        assert_eq!(renderer.counters().visible_sections, 1);
        assert_eq!(
            renderer
                .terrain_opaque
                .iter()
                .filter(|mesh| renderer.terrain_mesh_is_visible(mesh))
                .count(),
            1
        );
        assert!(renderer
            .terrain_opaque
            .iter()
            .find(|mesh| mesh.owner.is_some_and(|owner| owner.chunk == east))
            .is_some_and(|mesh| renderer.terrain_mesh_is_visible(mesh)));
        assert!(renderer
            .terrain_opaque
            .iter()
            .find(|mesh| mesh.owner.is_some_and(|owner| owner.chunk == west))
            .is_some_and(|mesh| !renderer.terrain_mesh_is_visible(mesh)));

        renderer.synchronize_terrain_section_mesh_layer_refs(
            3,
            [
                (west, &[(4, 1)][..], &layers, [0.0; 3]),
                (east, &[(4, 1)][..], &layers, [0.0; 3]),
                (north, &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
        assert_eq!(
            renderer.counters().visible_sections,
            1,
            "unchanged residency keeps the current graph result"
        );

        renderer.synchronize_terrain_section_mesh_layer_refs(
            3,
            [
                (west, &[(4, 2)][..], &layers, [0.0; 3]),
                (east, &[(4, 1)][..], &layers, [0.0; 3]),
                (north, &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
        assert_eq!(renderer.counters().visible_sections, 1);
        assert_eq!(
            renderer
                .terrain_opaque
                .iter()
                .filter(|mesh| renderer.terrain_mesh_is_visible(mesh))
                .count(),
            1,
            "recompiling a culled owner cannot bypass the current native projection"
        );
        assert!(renderer
            .terrain_opaque
            .iter()
            .find(|mesh| mesh.owner.is_some_and(|owner| owner.chunk == north))
            .is_some_and(|mesh| !renderer.terrain_mesh_is_visible(mesh)));
    }

    #[test]
    fn terrain_visibility_filter_changes_deterministic_frame_pixels() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(64, 64) else {
            return;
        };
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        renderer
            .upload_terrain_texture_atlas(1, 1, &[255; 4])
            .expect("white terrain atlas");
        let vertex = |position| {
            let mut vertex = terrain_vertex(position);
            vertex.normal = [0.0, 0.0, -1.0];
            vertex.uv = [0.5; 2];
            vertex.light = [1.0; 2];
            vertex.tint = [1.0, 0.0, 0.0];
            vertex
        };
        let layers = TerrainSectionMeshLayers {
            opaque: vec![TerrainSectionMesh {
                section_y: 0,
                mesh: TerrainMesh {
                    vertices: vec![
                        vertex([-1.0, -1.0, 2.0]),
                        vertex([1.0, -1.0, 2.0]),
                        vertex([1.0, 1.0, 2.0]),
                        vertex([-1.0, 1.0, 2.0]),
                    ],
                    // Both windings keep this readback independent of the
                    // backend's viewport-Y front-face convention.
                    indices: vec![0, 1, 2, 2, 3, 0, 0, 3, 2, 2, 1, 0],
                    source_sections: 1,
                    opaque_faces: 2,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        let chunk = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: 0,
            chunk_z: 0,
        };
        let owner = TerrainSectionMeshOwner {
            chunk,
            section_y: 0,
        };
        renderer.synchronize_terrain_section_mesh_layer_refs(
            1,
            [(chunk, &[(0, 1)][..], &layers, [0.0; 3])],
        );
        renderer.set_terrain_section_states(
            1,
            &[TerrainSectionRenderState {
                owner,
                visibility: 1.0,
                nearby: true,
            }],
        );
        renderer.set_terrain_section_states(1, &[]);
        assert!(renderer.terrain_section_visibility.contains_key(&owner));
        assert_eq!(renderer.terrain_section_uniform_scan_count, 0);
        let visible = renderer
            .render_offscreen_frame()
            .expect("visible terrain frame");

        renderer.set_terrain_section_states(
            2,
            &[TerrainSectionRenderState {
                owner,
                visibility: 0.0,
                nearby: true,
            }],
        );
        assert_eq!(renderer.terrain_section_uniform_scan_count, 1);
        let faded = renderer
            .render_offscreen_frame()
            .expect("zero-visibility terrain frame");

        renderer.set_terrain_section_states(3, &[]);
        assert_eq!(renderer.terrain_section_uniform_scan_count, 2);
        let hidden = renderer
            .render_offscreen_frame()
            .expect("culled terrain frame");

        let visible_center = visible.pixel(32, 32);
        let faded_center = faded.pixel(32, 32);
        let hidden_center = hidden.pixel(32, 32);
        assert!(
            visible_center[0] > visible_center[2],
            "visible owner should draw the red terrain quad, got {visible_center:?}"
        );
        assert!(
            faded_center[0] < visible_center[0],
            "section visibility uniform should mix terrain toward fog color: {faded_center:?} vs {visible_center:?}"
        );
        assert!(
            hidden_center[2] > hidden_center[0],
            "culled owner should expose the blue clear color, got {hidden_center:?}"
        );
        assert_ne!(visible.rgba, hidden.rgba);
    }

    #[test]
    fn incremental_terrain_residency_preserves_unchanged_sibling_section_buffers() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let layers = TerrainSectionMeshLayers {
            opaque: [4, 5]
                .into_iter()
                .map(|section_y| TerrainSectionMesh {
                    section_y,
                    mesh: TerrainMesh {
                        vertices: vec![
                            terrain_vertex([0.0, section_y as f32 * 16.0, 0.0]),
                            terrain_vertex([1.0, section_y as f32 * 16.0, 0.0]),
                            terrain_vertex([0.0, section_y as f32 * 16.0 + 1.0, 0.0]),
                        ],
                        indices: vec![0, 1, 2],
                        source_sections: 1,
                        opaque_faces: 1,
                        ..TerrainMesh::default()
                    },
                })
                .collect(),
            source_sections: 2,
            ..TerrainSectionMeshLayers::default()
        };
        let chunk = TerrainChunkMeshOwner {
            level_generation: 7,
            chunk_x: 0,
            chunk_z: 0,
        };

        renderer.synchronize_terrain_section_mesh_layer_refs(
            2,
            [(chunk, &[(4, 1), (5, 1)][..], &layers, [0.0; 3])],
        );
        let one_section_bytes = renderer.counters().upload_bytes / 2;
        assert_eq!(renderer.terrain_opaque.len(), 2);

        renderer.synchronize_terrain_section_mesh_layer_refs(
            2,
            [(chunk, &[(4, 2), (5, 1)][..], &layers, [0.0; 3])],
        );
        assert_eq!(renderer.counters().terrain_changed_owners, 1);
        assert_eq!(renderer.counters().terrain_uploaded_owners, 1);
        assert_eq!(renderer.counters().terrain_removed_owners, 1);
        assert_eq!(renderer.counters().upload_bytes, one_section_bytes);
        assert_eq!(renderer.counters().terrain_full_replacements, 0);
        assert_eq!(renderer.terrain_opaque.len(), 2);
        assert!(renderer
            .terrain_opaque
            .iter()
            .any(|resident| resident.owner.is_some_and(|owner| owner.section_y == 5)));
    }

    #[test]
    fn translucent_camera_resort_uploads_indices_without_replacing_vertices() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
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
        let layers = TerrainSectionMeshLayers {
            translucent: vec![TerrainSectionMesh {
                section_y: 0,
                mesh: TerrainMesh {
                    vertices,
                    indices: vec![4, 5, 6, 6, 7, 4, 0, 1, 2, 2, 3, 0],
                    source_sections: 1,
                    translucent_faces: 2,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        let owner = TerrainChunkMeshOwner {
            level_generation: 1,
            chunk_x: 0,
            chunk_z: 0,
        };
        renderer.synchronize_terrain_section_mesh_layer_refs(
            1,
            [(owner, &[(0, 1)][..], &layers, [0.5, 0.5, -4.0])],
        );
        let section_owner = TerrainSectionMeshOwner {
            chunk: owner,
            section_y: 0,
        };
        renderer.set_terrain_section_states(
            1,
            &[TerrainSectionRenderState {
                owner: section_owner,
                visibility: 1.0,
                nearby: true,
            }],
        );
        assert_eq!(
            renderer.terrain_nearby_visible_section_order,
            [section_owner]
        );
        assert_eq!(
            renderer.terrain_translucent_indices.get(&section_owner),
            Some(&0)
        );
        let vertex_upload_bytes = renderer.counters().terrain_vertex_upload_bytes;
        renderer.terrain_translucent_order.reserve(64);
        let section_order_capacity = renderer.terrain_translucent_order.capacity();

        renderer.set_camera_pose(Some(CameraPose {
            position: [0.5, 0.5, 4.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));

        assert_eq!(renderer.counters().terrain_translucent_resort_owners, 1);
        assert_eq!(
            renderer.counters().terrain_translucent_resort_index_bytes,
            12 * size_of::<u32>() as u64
        );
        assert_eq!(
            renderer.counters().terrain_vertex_upload_bytes,
            vertex_upload_bytes
        );
        assert_eq!(
            renderer.terrain_translucent_order.capacity(),
            section_order_capacity,
            "camera motion must not rebuild native's authoritative visible-section order"
        );
        assert_eq!(renderer.counters().terrain_changed_owners, 1);
        assert_eq!(renderer.counters().terrain_full_replacements, 0);
    }

    #[test]
    fn translucent_camera_resort_drains_block_boundary_work_over_updates() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let vertices = vec![
            terrain_vertex([0.0, 0.0, 0.0]),
            terrain_vertex([1.0, 0.0, 0.0]),
            terrain_vertex([1.0, 1.0, 0.0]),
            terrain_vertex([0.0, 1.0, 0.0]),
        ];
        let layers = TerrainSectionMeshLayers {
            translucent: (0..5)
                .map(|section_y| TerrainSectionMesh {
                    section_y,
                    mesh: TerrainMesh {
                        vertices: vertices.clone(),
                        indices: vec![0, 1, 2, 2, 3, 0],
                        source_sections: 1,
                        translucent_faces: 1,
                        ..TerrainMesh::default()
                    },
                })
                .collect(),
            source_sections: 5,
            ..TerrainSectionMeshLayers::default()
        };
        let chunk = TerrainChunkMeshOwner {
            level_generation: 1,
            chunk_x: 0,
            chunk_z: 0,
        };
        let revisions = [(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)];
        renderer.synchronize_terrain_section_mesh_layer_refs(
            5,
            [(chunk, &revisions[..], &layers, [0.5, 32.5, -4.0])],
        );
        renderer.set_terrain_section_states(
            1,
            &(0..5)
                .map(|section_y| TerrainSectionRenderState {
                    owner: TerrainSectionMeshOwner { chunk, section_y },
                    visibility: 1.0,
                    nearby: true,
                })
                .collect::<Vec<_>>(),
        );

        let mut completed = 0;
        for x in [0.5, 0.6, 0.7, 0.8, 0.9] {
            renderer.set_camera_pose(Some(CameraPose {
                position: [x, 32.5, 4.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            }));
            let resorts = renderer.counters().terrain_translucent_resort_owners;
            assert!(resorts <= MAX_TRANSLUCENT_RESORTS_PER_CAMERA_UPDATE);
            completed += resorts;
        }

        assert_eq!(completed, 5);
        assert!(renderer.terrain_translucent.iter().all(|mesh| mesh
            .translucent_sort
            .as_ref()
            .is_some_and(|sort| sort.pending_camera.is_none())));
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
        let indices = vec![
            0, 2, 1, 0, 3, 2, // first quad uses BBB's reversed local winding
            4, 5, 6, 4, 6, 7,
        ];
        let mut sort =
            TerrainTranslucentSortState::from_vertices(&vertices, &indices, Some([0, 0, -1]))
                .expect("quad vertices and indices create sort state");

        assert_eq!(
            sort.indices_for_camera_if_needed([0.5, 0.5, -4.0], [0, 0, -1], false, true,),
            None
        );
        assert_eq!(
            sort.indices_for_camera_if_needed([0.5, 0.5, 4.0], [0, 0, 1], true, true),
            Some(vec![0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7])
        );
        assert_eq!(
            sort.indices_for_camera_if_needed([0.5, 0.5, -4.0], [0, 0, -1], true, true),
            Some(vec![4, 5, 6, 4, 6, 7, 0, 2, 1, 0, 3, 2])
        );
    }

    #[test]
    fn translucent_sort_state_uses_vanilla_block_and_point_of_view_triggers() {
        let vertices = vec![
            terrain_vertex([0.0, 0.0, 0.0]),
            terrain_vertex([1.0, 0.0, 0.0]),
            terrain_vertex([1.0, 1.0, 0.0]),
            terrain_vertex([0.0, 1.0, 0.0]),
        ];
        let mut sort = TerrainTranslucentSortState::from_vertices(
            &vertices,
            &[0, 1, 2, 2, 3, 0],
            Some([1, 1, 1]),
        )
        .expect("one translucent quad");

        assert_eq!(
            sort.indices_for_camera_if_needed([0.25, 0.5, 0.5], [1, 1, 1], false, true),
            None,
            "sub-block interpolation alone does not schedule a Vanilla resort"
        );
        assert_eq!(
            sort.indices_for_camera_if_needed([1.25, 0.5, 0.5], [1, 1, 1], true, false),
            None,
            "far, non-axis-aligned sections wait for their round-robin point-of-view change"
        );
        assert!(sort
            .indices_for_camera_if_needed([1.25, 0.5, 0.5], [0, 1, 1], true, false)
            .is_some());
        assert!(sort
            .indices_for_camera_if_needed([2.25, 0.5, 0.5], [0, 1, 1], true, false)
            .is_some());
    }

    #[test]
    fn translucent_point_of_view_and_nearby_match_section_coordinates() {
        let owner = TerrainSectionMeshOwner {
            chunk: TerrainChunkMeshOwner {
                level_generation: 1,
                chunk_x: -2,
                chunk_z: 4,
            },
            section_y: 7,
        };

        assert_eq!(
            translucency_point_of_view([-31.5, 127.9, 64.0], owner),
            [0, 0, 0]
        );
        assert!(translucent_section_is_nearby([-0.1, 159.9, 96.0], owner));
        assert!(!translucent_section_is_nearby([16.0, 159.9, 96.0], owner));
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
    fn translucent_sections_reverse_the_exact_visible_traversal() {
        let owner = |chunk_x, section_y| TerrainSectionMeshOwner {
            chunk: TerrainChunkMeshOwner {
                level_generation: 7,
                chunk_x,
                chunk_z: 0,
            },
            section_y,
        };
        let near = owner(0, 4);
        let middle = owner(1, 4);
        let far = owner(2, 4);
        let invisible = owner(3, 4);

        // Resident storage order is deliberately unrelated to visibility order.
        let owners = [Some(middle), Some(invisible), Some(far), Some(near), None];
        assert_eq!(
            translucent_section_draw_order_from_visible(&owners, &[near, middle, far]),
            vec![2, 0, 3, 1, 4]
        );
        // The order, not merely set membership, is part of the rendering fact.
        assert_eq!(
            translucent_section_draw_order_from_visible(&owners, &[far, middle, near]),
            vec![3, 0, 2, 1, 4]
        );
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
