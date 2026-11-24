use bbb_client_clock::{Elapsed as _, Instant};
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap, HashSet},
    hash::{BuildHasher, Hasher},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result};
use bbb_render_types::{
    DynamicLightsMode, DynamicPointLight, EntityDefaultPlayerSkin, StandaloneTextureSampling,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::frame_buffers::{FrameDataBuffer, StagedCommands};
pub(crate) use crate::gpu::effective_terrain_max_anisotropy;
pub use crate::gpu::TerrainTextureFiltering;
use crate::texture_sampling::{create_nearest_clamp_sampler, create_texture_bind_group};
use crate::{
    block_destroy::{BlockDestroyOverlay, BlockDestroyOverlaysGpu},
    camera::{
        sanitize_lightmap_block_factor, sanitize_lightmap_brightness_factor, CameraHurtEffect,
        CameraPose, CameraSpinningEffect, CameraUniform, CameraViewBobEffect, ClearColor,
        FogEnvironment, LightmapEnvironment, TerrainBounds, VANILLA_DEFAULT_GLINT_SPEED,
    },
    clouds::{
        cloud_mesh_key, cloud_uniform, create_cloud_bind_group, create_cloud_bind_group_layout,
        create_cloud_gpu, create_cloud_target, create_cloud_texture_data, CloudEnvironment,
        CloudFrame, CloudGpu, CloudShape, CloudTarget, CloudTextureData, CloudTextureImage,
        CloudUniform,
    },
    counters::{RendererCounters, TerrainMeshWorkerCounters},
    debug_filled_boxes::{
        sanitize_debug_filled_boxes, sanitize_debug_filled_rects, DebugFilledBox, DebugFilledRect,
    },
    debug_text_billboards::{sanitize_debug_text_billboards, DebugTextBillboard},
    entity_models::{
        create_entity_model_armor_entity_glint_pipeline,
        create_entity_model_beacon_beam_opaque_pipeline,
        create_entity_model_beacon_beam_translucent_pipeline,
        create_entity_model_dragon_rays_depth_pipeline, create_entity_model_dragon_rays_pipeline,
        create_entity_model_end_crystal_beam_pipeline, create_entity_model_end_gateway_pipeline,
        create_entity_model_end_portal_pipeline, create_entity_model_entity_glint_pipeline,
        create_entity_model_guardian_beam_pipeline, create_entity_model_leash_pipeline,
        create_entity_model_outline_cull_pipeline, create_entity_model_outline_pipeline,
        create_entity_model_scroll_additive_pipeline, create_entity_model_scroll_pipeline,
        create_entity_model_water_mask_pipeline, EntityDynamicPlayerSkinAtlasGpu,
        EntityDynamicPlayerTextureAtlasGpu, EntityFlameSurface, EntityModelDrawRange,
        EntityModelMeshGpu, EntityModelTextureAtlasGpu, EntityModelTexturedDrawRange,
        EntityModelTranslucentDrawRange, EntityShadowSurface, EntityShadowTextureGpu,
        FirstPersonPlayerArm, HudEntityPreviewPipTarget,
    },
    frame_flight::{RenderSurface, TerrainCommandFinisher},
    gpu::{
        create_depth_target, create_terrain_atlas_gpu, create_terrain_atlas_mips_gpu,
        create_terrain_bind_group_layout, write_terrain_atlas_gpu,
        write_terrain_atlas_mip_patches_gpu, write_terrain_atlas_mips_gpu, DepthTarget,
        FrameCameraBuffers, TerrainAtlasGpu, TerrainCorePipelines,
    },
    gpu_timing::{optional_gpu_timing_features, GpuFrameTiming},
    hud::{
        create_hud_bind_group_layout, HudChatContext, HudChatLine, HudChatMessage,
        HudCrosshairAttackIndicator, HudDigitGlyph, HudFontGlyphBook, HudFontGlyphMap,
        HudHotbarMode, HudInventoryScreen, HudItemIcon, HudObfuscatedGlyphPool, HudSpriteGpu,
        HUD_HOTBAR_SLOTS,
    },
    item_entities::{create_item_entity_pipeline, ItemEntityAtlasGpu, ItemEntityBillboard},
    item_models::{
        create_display_text_background_pipeline, create_display_text_pipeline,
        create_display_text_see_through_pipeline, create_entity_shadow_pipeline,
        create_resident_item_model_cutout_no_cull_pipeline,
        create_resident_item_model_cutout_z_offset_pipeline,
        create_resident_item_model_end_gateway_pipeline,
        create_resident_item_model_end_portal_pipeline, create_resident_item_model_glint_pipeline,
        create_resident_item_model_outline_pipeline, create_resident_item_model_pattern_pipeline,
        create_resident_item_model_solid_no_cutout_pipeline,
        create_resident_item_model_translucent_no_cull_pipeline,
        create_resident_item_model_z_offset_forward_pipeline, DisplayTextSurface,
        EntityNameTagSurface, FirstPersonMapBackgroundAtlasGpu, GlintTextureGpu,
        ItemFrameMapAtlasGpu, ItemFrameMapDecorationAtlasGpu, ItemFrameMapTextFontAtlasGpu,
        ItemModelResidency, MapSurfaceResidency, SignTextOutlineContext, SignTextSurface,
    },
    lightmap::{
        create_lightmap_bind_group_layout, create_lightmap_gpu,
        create_lightmap_sample_bind_group_layout, LightmapGpu,
    },
    outline::{
        create_entity_outline_bind_group_layout, create_entity_outline_target, EntityOutlineTarget,
    },
    particles::{ParticleAtlasGpu, ParticleSpriteUv},
    player_skin::{DynamicEntityTextureImage, DynamicPlayerSkinImage},
    post_effect_graph::{PostEffectGraphRuntime, PostEffectGraphRuntimeResize},
    screen_blur::{
        sanitize_menu_background_blurriness, VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
    },
    selection::{
        create_chunk_border_always_on_top_pipeline, create_selection_pipeline,
        create_translucent_debug_line_pipeline, sanitize_debug_lines, BlockSelectionGpu,
        BlockSelectionRenderState, DebugLine, SelectionOutline, SelectionOutlineGpu,
    },
    sky::{
        create_celestial_atlas_gpu, create_end_sky_texture_gpu, CelestialTextureImage,
        SkyEnvironment, SkyGpu,
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
    vignette::{create_vignette_bind_group_layout, VignetteTextureGpu},
    weather::{
        create_weather_texture_gpu, WeatherRenderState, WeatherTextureGpu, WeatherTextureImage,
        WeatherTextureKind,
    },
    world_border::{
        create_world_border_texture_gpu, WorldBorderRenderState, WorldBorderTextureGpu,
    },
};

mod entity_meshes;
mod frame_targets;
mod pack_resources;
mod pipelines;
mod profiling;
mod scene_state;
mod terrain_residency;
mod terrain_state;
pub(crate) use entity_meshes::{EntityMeshLane, EntityMeshes};
pub(crate) use frame_targets::{FrameTargetLayouts, FrameTargets};
pub(crate) use pack_resources::PackContext;
pub(crate) use pack_resources::PackResources;
pub(crate) use pipelines::{CorePipelineLayouts, CorePipelines, SelectedCoreShaders};
pub use profiling::*;
pub use scene_state::*;
pub(crate) use terrain_residency::{TerrainLayer, TerrainResidency};
pub use terrain_state::*;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RendererResizePrepareStage {
    PostEffects,
    FrameTargets,
}

struct RendererResizePlan {
    size: PhysicalSize<u32>,
    post_effect_graphs: Option<PostEffectGraphRuntimeResize>,
    targets: FrameTargets,
}

pub struct Renderer {
    pub(crate) entity_pose_pipelines: Option<crate::entity_models::EntityPosePipelines>,
    pub(super) terrain_finish: Option<TerrainCommandFinisher>,
    pub(super) surface: RenderSurface,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) config: wgpu::SurfaceConfiguration,
    /// The present modes the surface exposed at construction, so
    /// `set_vsync_enabled` can renegotiate `updateVsync`
    /// without recreating the adapter.
    pub(super) present_modes: Vec<wgpu::PresentMode>,
    /// `Options.enableVsync`: re-applied on fullscreen changes and forced off
    /// by `Window.updateVsync`. Winit exposes no synchronous fullscreen
    /// failure, so `Minecraft.onFullscreenError` has no trigger on this
    /// backend.
    pub(super) vsync_enabled: bool,
    // Window.setGuiScale retains ceil dimensions for layout; projection uses physical/scale.
    pub(super) gui_surface_size: PhysicalSize<u32>,
    pub(super) gui_scale: u32,
    #[cfg(test)]
    pub(super) camera_uniform_writes: std::cell::Cell<u64>,
    pub(super) size: PhysicalSize<u32>,
    pub(super) clear: ClearColor,
    pub(super) level_frame_route: LevelFrameRoute,
    pub(super) counters: RendererCounters,
    #[cfg(target_arch = "wasm32")]
    pub(super) pending_web_screenshot:
        Option<Box<dyn FnOnce(Result<crate::CapturedScreenshot>) + 'static>>,
    #[cfg(feature = "lifecycle-soak")]
    pub(super) lifecycle_soak_surface_faults: std::collections::VecDeque<wgpu::SurfaceStatus>,
    pub(super) started_at: Instant,
    pub(super) gpu_timing: Option<GpuFrameTiming>,
    /// Enables detailed CPU phase timestamps together with GPU diagnostics.
    /// Normal frames omit the extra render-call and submission timestamps.
    pub(super) frame_phase_profiling_enabled: bool,
    pub(super) submission_profiling_enabled: bool,
    pub(super) last_frame_profile: RendererFrameProfile,
    pub(super) input_latency: RendererInputLatencyState,
    pub(super) last_frame_started_at: Instant,
    pub(super) shader_game_time_ticks: f64,
    /// Vanilla `optionsRenderState.glintSpeed` (`Options.glintSpeed`).
    pub(super) glint_speed: f64,
    /// Vanilla `optionsRenderState.glintStrength` (`Options.glintStrength`),
    /// the `GlobalSettingsUniform` glint alpha.
    pub(super) glint_alpha: f32,
    pub(super) current_post_effect_id: Option<String>,
    pub(super) camera_post_effect_active: bool,
    pub(super) post_effect_graphs: Option<PostEffectGraphRuntime>,
    pub(super) menu_background_blurriness: f32,
    pub(super) hud_text_background_opacity: f32,
    pub(super) hud_background_for_chat_only: bool,
    pub(super) hide_gui: bool,
    pub(super) hud_screen_blur: bool,
    pub(super) transparency_combine_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) transparency_hand_pipeline: wgpu::RenderPipeline,
    pub(super) transparency_combine_pipeline: wgpu::RenderPipeline,
    pub(super) transparency_blit_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) transparency_blit_pipeline: wgpu::RenderPipeline,
    pub(super) targets: FrameTargets,
    pub(super) pipelines: CorePipelines,
    pub(super) pack: PackResources,
    pub(super) core_shader_generation: Option<u64>,
    pub(super) terrain_texture_filtering: TerrainTextureFiltering,
    pub(super) terrain_max_anisotropy_bit: u32,
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
    pub(super) hud_geometry: crate::hud::HudFrameGeometry,
    pub(super) hud_generation: u64,
    /// Geometry of the frame's `&self` item-model, text, shadow and screen-effect draws.
    pub(super) lightmap: LightmapGpu,
    pub(super) lightmap_dirty: bool,
    pub(super) lightmap_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) lightmap_hud_bind_group: wgpu::BindGroup,
    pub(super) _lightmap_hud_sampler: wgpu::Sampler,
    pub(super) lightmap_sample_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) entity_outline_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) cloud_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) cloud_bind_group: wgpu::BindGroup,
    pub(super) cloud_quad_indices: Option<wgpu::Buffer>,
    pub(super) hud_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) vignette_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) screen_effect_quads: Vec<crate::screen_effects::ScreenEffectQuad>,
    pub(super) screen_effect_projection: Option<(f32, f32)>,
    pub(super) screen_effect_vertices: FrameDataBuffer,
    pub(super) screen_effect_batches: Vec<crate::screen_effects::ScreenEffectDrawBatch>,
    pub(super) screen_effect_underwater: Option<crate::screen_effects::ScreenEffectUnderwaterGpu>,
    pub(super) hud_white_pixel: HudSpriteGpu,
    #[cfg(test)]
    pub(super) test_gui_sprite_images: Vec<bbb_client_assets::SpriteImage>,
    pub(super) terrain_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) camera_buffers: FrameCameraBuffers,
    pub(super) pending_camera_uniforms: std::cell::Cell<Option<[CameraUniform; 3]>>,
    pub(super) pending_cloud_uniform: std::cell::Cell<Option<CloudUniform>>,
    pub(super) hud_hotbar_block_item_models: Vec<Option<crate::item_models::HudBlockItemModel>>,
    pub(super) hud_offhand_slot: Option<crate::hud::HudOffhandSlot>,
    pub(super) hud_offhand_block_item_model: Option<crate::item_models::HudBlockItemModel>,
    pub(super) hud_hotbar_attack_indicator: crate::hud::HudHotbarAttackIndicator,
    pub(super) frame_uploads: StagedCommands,
    terrain_uploads: StagedCommands,
    pub(super) terrain: TerrainResidency,
    pub(super) camera_pose: Option<CameraPose>,
    pub(super) camera_hurt_effect: Option<CameraHurtEffect>,
    pub(super) camera_view_bob_effect: Option<CameraViewBobEffect>,
    pub(super) camera_spinning_effect: Option<CameraSpinningEffect>,
    pub(super) camera_fov_degrees: f32,
    pub(super) camera_hud_fov_degrees: f32,
    pub(super) lightmap_environment: LightmapEnvironment,
    pub(super) fog_environment: FogEnvironment,
    pub(super) sky: SkyGpu,
    pub(super) cloud_environment: CloudEnvironment,
    pub(super) cloud_frame: CloudFrame,
    pub(super) cloud_shape: CloudShape,
    /// Vanilla `CloudStatus.OFF` (`renderClouds: "false"`): `LevelRenderer`
    /// skips the cloud pass entirely.
    pub(super) clouds_enabled: bool,
    pub(super) block_destroy_overlays: BlockDestroyOverlaysGpu,
    pub(super) block_entity_crumbling_index_count: u32,
    pub(super) entity_dynamic_player_skin_atlas: Option<EntityDynamicPlayerSkinAtlasGpu>,
    pub(super) entity_dynamic_player_skin_images: Vec<DynamicPlayerSkinImage>,
    pub(super) entity_dynamic_player_texture_atlas: Option<EntityDynamicPlayerTextureAtlasGpu>,
    pub(super) entity_dynamic_player_texture_images: Vec<DynamicEntityTextureImage>,
    pub(super) dynamic_entity_texture_generation: Option<(u64, u64, bool)>,
    pub(crate) entity_meshes_staged_unsubmitted: bool,
    pub(super) entity_model_instances: Vec<crate::EntityModelInstance>,
    pub(super) entity_flame_surfaces: Vec<EntityFlameSurface>,
    pub(super) entity_shadow_surfaces: Vec<EntityShadowSurface>,
    pub(super) first_person_player_arms: Vec<FirstPersonPlayerArm>,
    pub(super) weather_render_state: WeatherRenderState,
    pub(super) world_border_render_state: WorldBorderRenderState,
    pub(super) item_entity_billboards: Vec<ItemEntityBillboard>,
    #[cfg(test)]
    pub(super) item_frame_map_surfaces: Vec<crate::item_models::ItemFrameMapSurface>,
    #[cfg(test)]
    pub(super) first_person_map_surfaces: Vec<crate::item_models::ItemFrameMapSurface>,
    #[cfg(test)]
    pub(super) first_person_map_background_surfaces:
        Vec<crate::item_models::FirstPersonMapBackgroundSurface>,
    #[cfg(test)]
    pub(super) item_frame_map_decoration_surfaces:
        Vec<crate::item_models::ItemFrameMapDecorationSurface>,
    #[cfg(test)]
    pub(super) first_person_map_decoration_surfaces:
        Vec<crate::item_models::ItemFrameMapDecorationSurface>,
    pub(super) sign_text_surfaces: Vec<SignTextSurface>,
    pub(super) visible_sign_text_positions: Vec<[i32; 3]>,
    pub(super) sign_text_outline_context: SignTextOutlineContext,
    pub(super) display_text_surfaces: Vec<DisplayTextSurface>,
    pub(super) entity_name_tag_surfaces: Vec<EntityNameTagSurface>,
    pub(super) block_selection: BlockSelectionGpu,
    pub(super) chunk_border_outline: SelectionOutlineGpu,
    pub(super) entity_scene_outline: SelectionOutlineGpu,
    pub(super) debug_filled_boxes: Vec<DebugFilledBox>,
    pub(super) debug_filled_rects: Vec<DebugFilledRect>,
    pub(super) debug_text_billboards: Vec<DebugTextBillboard>,
    pub(super) debug_lines: Vec<DebugLine>,
    /// Revision-keyed GameTest values follow the per-frame debug renderer
    /// values in Vanilla's gizmo insertion order.
    pub(super) game_test_highlight_boxes: Vec<DebugFilledBox>,
    pub(super) game_test_highlight_text_billboards: Vec<DebugTextBillboard>,
    pub(super) hud_camera_overlays: Vec<crate::hud::HudCameraOverlay>,
    pub(super) hud_portal_overlay: Option<crate::hud::HudPortalOverlay>,
    pub(super) hud_confusion_overlay: Option<crate::hud::HudConfusionOverlay>,
    pub(super) hud_crosshair_visible: bool,
    pub(super) hud_crosshair_attack_indicator: HudCrosshairAttackIndicator,
    pub(super) hud_hotbar_item_icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS],
    pub(super) hud_now_playing_toast: Option<crate::hud::HudNowPlayingToast>,
    pub(super) hud_toasts: Vec<crate::hud::HudToast>,
    pub(super) hud_screen_present: bool,
    pub(super) hud_ingame_ui_present: bool,
    pub(super) hud_inventory_screen: Option<HudInventoryScreen>,
    pub(super) hud_inventory_bundle_tooltip: Option<crate::hud::HudBundleTooltip>,
    pub(super) hud_sign_editor_screen: Option<crate::hud::HudSignEditorScreen>,
    pub(super) hud_command_block_editor_screen: Option<crate::hud::HudCommandBlockEditorScreen>,
    pub(super) hud_structure_block_editor_screen: Option<crate::hud::HudStructureBlockEditorScreen>,
    pub(super) hud_jigsaw_block_editor_screen: Option<crate::hud::HudJigsawBlockEditorScreen>,
    pub(super) hud_test_block_editor_screen: Option<crate::hud::HudTestBlockEditorScreen>,
    pub(super) hud_test_instance_block_editor_screen:
        Option<crate::hud::HudTestInstanceBlockEditorScreen>,
    pub(super) hud_pause_screen: Option<crate::hud::HudPauseScreen>,
    pub(super) hud_social_interactions_screen: Option<crate::hud::HudSocialInteractionsScreen>,
    pub(super) hud_stats_screen: Option<crate::hud::HudStatsScreen>,
    pub(super) hud_game_rules_screen: Option<crate::hud::HudGameRulesScreen>,
    pub(super) hud_command_confirmation: Option<crate::hud::HudCommandConfirmation>,
    pub(super) hud_death_screen: Option<crate::hud::HudDeathScreen>,
    pub(super) hud_disconnected_screen: Option<crate::hud::HudDisconnectedScreen>,
    pub(super) hud_level_loading_screen: Option<crate::hud::HudLevelLoadingScreen>,
    pub(super) hud_game_event_screen: Option<crate::hud::HudGameEventScreen>,
    pub(super) hud_book_screen: Option<crate::hud::HudBookScreen>,
    pub(super) hud_debug_options_screen: Option<crate::hud::HudDebugOptionsScreen>,
    pub(super) hud_sound_subtitles: crate::hud::HudSoundSubtitles,
    /// One persistent PIP target per sanitized `hud_inventory_screen` entity preview, indexed by
    /// preview order (vanilla `PictureInPictureRenderer` private color+depth textures).
    pub(super) hud_entity_preview_pip_targets: Vec<HudEntityPreviewPipTarget>,
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
    pub(super) hud_chat_options: crate::hud::HudChatOptions,
    pub(super) hud_chat_context: HudChatContext,
    pub(super) hud_chat_appended_lines: Option<crate::hud::HudChatAppendedLines>,
    pub(super) hud_chat_restrictions_screen: Option<crate::hud::HudChatRestrictionsScreen>,
    pub(super) hud_command_suggestions: Option<crate::hud::HudCommandSuggestions>,
    pub(super) hud_debug_overlay: Option<crate::hud::HudDebugOverlay>,
    /// This frame's projected boss bars plus the 182x5 sprite sheets (one
    /// texture per vanilla `boss_bar/*` GUI sprite, indexed by the enum
    /// ordinals like `BossHealthOverlay`'s sprite arrays).
    pub(super) hud_boss_bars: Vec<crate::hud::HudBossBar>,
    pub(super) data_driven_rigs: Arc<[crate::entity_models::BakedDataDrivenRig]>,
    pub(super) dynamic_lights_mode: DynamicLightsMode,
    pub(super) dynamic_lights: Vec<DynamicPointLight>,
    pub(super) lod_terrain: crate::scene::lod_terrain::LodTerrainGpu,
    pub(super) lod_fog_end: f32,
    pub(super) particle_frame: crate::particles::ParticleFrame,
    pub(super) resource_reload_checkpoint:
        Option<crate::resource_reload::RendererResourceReloadCheckpoint>,
}

static FORCE_SOFTWARE_ADAPTER: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn set_force_software_adapter(enabled: bool) {
    FORCE_SOFTWARE_ADAPTER.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn force_software_adapter() -> bool {
    FORCE_SOFTWARE_ADAPTER.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var("BBB_SOFTWARE_GPU").as_deref() == Ok("1")
}

pub(crate) fn renderer_instance_backends() -> wgpu::Backends {
    if cfg!(target_arch = "wasm32") {
        wgpu::Backends::BROWSER_WEBGPU
    } else if cfg!(windows) {
        wgpu::Backends::from_env().unwrap_or(wgpu::Backends::DX12)
    } else {
        wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY)
    }
}

pub(crate) fn renderer_instance() -> wgpu::Instance {
    let mut flags = wgpu::InstanceFlags::from_build_config();
    if !cfg!(debug_assertions) {
        // Release GPU captures can restore driver markers with WGPU_DISCARD_HAL_LABELS=0.
        let labels = wgpu::InstanceFlags::DISCARD_HAL_LABELS;
        flags |= labels.with_env() & labels;
    }
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: renderer_instance_backends(),
        flags,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    })
}

pub(crate) const RENDERER_MEMORY_HINTS: wgpu::MemoryHints = wgpu::MemoryHints::Manual {
    suballocated_device_memory_block_size: (8 << 20)..terrain_residency::TERRAIN_PAGE_BYTES as u64,
};

pub(crate) fn renderer_device_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    let adapter_features = adapter.features();
    let mut features = optional_gpu_timing_features(adapter_features);
    if adapter
        .get_downlevel_capabilities()
        .flags
        .contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION)
    {
        features |= adapter_features & terrain_residency::TERRAIN_MDI_FEATURES;
    }
    features
}

fn choose_present_mode(
    present_modes: &[wgpu::PresentMode],
    vsync_enabled: bool,
) -> Result<wgpu::PresentMode> {
    if cfg!(target_arch = "wasm32") {
        // wgpu's WebGPU backend only permits Fifo / AutoVsync / AutoNoVsync.
        let preferred = if vsync_enabled {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        return present_modes
            .iter()
            .copied()
            .find(|mode| *mode == preferred)
            .or_else(|| {
                present_modes
                    .iter()
                    .copied()
                    .find(|mode| *mode == wgpu::PresentMode::Fifo)
            })
            .or_else(|| {
                present_modes.iter().copied().find(|mode| {
                    matches!(
                        mode,
                        wgpu::PresentMode::Fifo
                            | wgpu::PresentMode::AutoVsync
                            | wgpu::PresentMode::AutoNoVsync
                    )
                })
            })
            .ok_or_else(|| anyhow!("surface has no WebGPU-compatible present modes"));
    }

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
        Self::new_with_vsync_and_terrain_filtering_for_fabulous(
            window,
            vsync_enabled,
            terrain_texture_filtering,
            false,
        )
        .await
    }

    pub async fn new_with_vsync_and_terrain_filtering_for_fabulous(
        window: &Window,
        vsync_enabled: bool,
        terrain_texture_filtering: TerrainTextureFiltering,
        fabulous_auto: bool,
    ) -> Result<Self> {
        let size = window.inner_size();
        let instance = renderer_instance();
        #[cfg(target_arch = "wasm32")]
        let surface = {
            use winit::platform::web::WindowExtWebSys as _;
            let canvas = window
                .canvas()
                .ok_or_else(|| anyhow!("web window has no HTMLCanvasElement"))?;
            instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?
        };
        #[cfg(not(target_arch = "wasm32"))]
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_display_and_window(
                window, window,
            )?)?
        };
        let software_adapter = force_software_adapter();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: if software_adapter {
                    wgpu::PowerPreference::None
                } else {
                    wgpu::PowerPreference::HighPerformance
                },
                compatible_surface: Some(&surface),
                force_fallback_adapter: software_adapter,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| anyhow!("no suitable GPU adapter: {error}"))?;
        let adapter_info = adapter.get_info();
        let required_features = renderer_device_features(&adapter);
        tracing::info!(
            adapter_name = %adapter_info.name,
            adapter_backend = %adapter_info.backend.to_str(),
            adapter_device_type = ?adapter_info.device_type,
            terrain_indirect_draws = required_features.contains(terrain_residency::TERRAIN_MDI_FEATURES),
            "renderer platform adapter selected"
        );
        let terrain_texture_filtering =
            resolve_fabulous_terrain_texture_filtering(terrain_texture_filtering, fabulous_auto);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                memory_hints: RENDERER_MEMORY_HINTS,
                label: Some("bbb-client-device"),
                required_features,
                required_limits: wgpu::Limits {
                    max_buffer_size: adapter.limits().max_buffer_size,
                    ..wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
                },
                ..Default::default()
            })
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = choose_format(&caps.formats)?;
        let present_mode = choose_present_mode(&caps.present_modes, vsync_enabled)?;
        tracing::info!(
            wgpu_version = WGPU_VERSION,
            wgpu_core_version = WGPU_CORE_VERSION,
            wgpu_hal_version = WGPU_HAL_VERSION,
            wgpu_types_version = WGPU_TYPES_VERSION,
            naga_version = NAGA_VERSION,
            upstream_metal_fence_commit = WGPU_METAL_FENCE_UPSTREAM_COMMIT,
            upstream_metal_fence_strategy = WGPU_METAL_FENCE_STRATEGY,
            "renderer dependency receipt"
        );
        tracing::info!(
            ?present_mode,
            ?format,
            surface_usages = ?caps.usages,
            presentation_copy = caps.usages.contains(wgpu::TextureUsages::COPY_DST),
            supported_present_modes = ?caps.present_modes,
            desired_maximum_frame_latency = 2,
            in_flight_submission_budget = 0,
            submission_fence_strategy = "surface-configuration",
            "renderer surface presentation selected"
        );
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | (caps.usages & wgpu::TextureUsages::COPY_DST),
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
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
            adapter
                .get_downlevel_capabilities()
                .flags
                .contains(wgpu::DownlevelFlags::VERTEX_STORAGE),
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
        vertex_storage: bool,
    ) -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            crate::frame_buffers::set_wasm_queue(&queue);
        }
        // Log uncaptured device validation errors instead of tripping wgpu's default
        // native abort/panic hook so pack validation or unexpected GPU states degrade cleanly.
        device.on_uncaptured_error(std::sync::Arc::new(|error: wgpu::Error| {
            tracing::error!(%error, "wgpu uncaptured device error");
        }));
        let format = config.format;
        let terrain_bind_group_layout = create_terrain_bind_group_layout(&device);
        let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
        let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
        let lightmap = create_lightmap_gpu(
            &device,
            &queue,
            &lightmap_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            LightmapEnvironment::default(),
        );
        let hud_bind_group_layout = create_hud_bind_group_layout(&device);
        let lightmap_hud_sampler =
            create_nearest_clamp_sampler(&device, "bbb-lightmap-hud-preview-sampler");
        let lightmap_hud_bind_group = create_texture_bind_group(
            &device,
            &hud_bind_group_layout,
            "bbb-lightmap-hud-preview-bind-group",
            &lightmap.view,
            &lightmap_hud_sampler,
        );
        let camera_buffers = FrameCameraBuffers::new(&device);
        let pack = PackResources::new(
            &PackContext {
                device: &device,
                queue: &queue,
                format,
                terrain_layout: &terrain_bind_group_layout,
                lightmap_sample_layout: &lightmap_sample_bind_group_layout,
                hud_layout: &hud_bind_group_layout,
                camera: camera_buffers.world(),
                hud3d_camera: camera_buffers.hud3d(),
                gui_item_camera: camera_buffers.gui(),
            },
            ItemModelResidency::new(),
            crate::entity_models::EntityGeometryResidency::new(&device),
        )?;
        let entity_outline_bind_group_layout = create_entity_outline_bind_group_layout(&device);
        let sky = SkyGpu::new(&device);
        let cloud_bind_group_layout = create_cloud_bind_group_layout(&device);
        let vignette_bind_group_layout = create_vignette_bind_group_layout(&device);
        let pipelines = CorePipelines::new(
            &CorePipelineLayouts {
                device: &device,
                format,
                terrain: &terrain_bind_group_layout,
                lightmap: &lightmap_bind_group_layout,
                lightmap_sample: &lightmap_sample_bind_group_layout,
                hud: &hud_bind_group_layout,
                gui_extent: [config.width as f32, config.height as f32],
                cloud: &cloud_bind_group_layout,
                vignette: &vignette_bind_group_layout,
                entity_outline: &entity_outline_bind_group_layout,
                sky: &sky,
                terrain_texture_filtering,
                dynamic_lights_mode: DynamicLightsMode::Off,
            },
            SelectedCoreShaders::default(),
        )?;
        let cloud_bind_group =
            create_cloud_bind_group(&device, &cloud_bind_group_layout, camera_buffers.cloud());
        let transparency_combine_bind_group_layout =
            create_transparency_combine_bind_group_layout(&device);
        let combine_pipeline = |hand_depth| {
            create_transparency_combine_pipeline(
                &device,
                format,
                &transparency_combine_bind_group_layout,
                hand_depth,
            )
        };
        let transparency_combine_pipeline = combine_pipeline(false);
        let transparency_hand_pipeline = combine_pipeline(true);
        let transparency_blit_bind_group_layout =
            create_transparency_blit_bind_group_layout(&device);
        let targets = FrameTargets::new(
            &device,
            format,
            config.width,
            config.height,
            &FrameTargetLayouts {
                transparency_blit: &transparency_blit_bind_group_layout,
                entity_outline: &entity_outline_bind_group_layout,
                transparency_combine: &transparency_combine_bind_group_layout,
            },
        );
        let transparency_blit_pipeline = create_transparency_blit_pipeline(
            &device,
            format,
            &transparency_blit_bind_group_layout,
        );
        let hud_white_pixel = HudSpriteGpu::solid(
            &device,
            &queue,
            &hud_bind_group_layout,
            [255, 255, 255, 255],
        )?;
        let gpu_timing = GpuFrameTiming::new(&device, &queue);

        let entity_pose_pipelines = vertex_storage.then(|| {
            crate::entity_models::EntityPosePipelines::new(
                &device,
                format,
                &terrain_bind_group_layout,
                &lightmap_sample_bind_group_layout,
            )
        });
        let frame_uploads = StagedCommands::new(&device, "bbb-client-clear", 256 * 1024);
        let terrain_uploads = StagedCommands::new(&device, "bbb-terrain-uploads", 512 * 1024);
        Ok(Self {
            entity_pose_pipelines,
            surface,
            device,
            queue,
            config,
            present_modes: Vec::new(),
            vsync_enabled: true,
            gui_surface_size: size,
            gui_scale: 1,
            #[cfg(test)]
            camera_uniform_writes: std::cell::Cell::new(0),
            size,
            clear: ClearColor::default(),
            level_frame_route: LevelFrameRoute::default(),
            counters: RendererCounters {
                width: size.width,
                height: size.height,
                gui_width: size.width,
                gui_height: size.height,
                ..RendererCounters::default()
            },
            #[cfg(target_arch = "wasm32")]
            pending_web_screenshot: None,
            #[cfg(feature = "lifecycle-soak")]
            lifecycle_soak_surface_faults: std::collections::VecDeque::new(),
            started_at: bbb_client_clock::now(),
            gpu_timing,
            frame_phase_profiling_enabled: false,
            submission_profiling_enabled: false,
            last_frame_profile: RendererFrameProfile::default(),
            input_latency: RendererInputLatencyState::default(),
            last_frame_started_at: bbb_client_clock::now(),
            shader_game_time_ticks: 0.0,
            glint_speed: VANILLA_DEFAULT_GLINT_SPEED,
            glint_alpha: crate::camera::VANILLA_DEFAULT_GLINT_STRENGTH,
            current_post_effect_id: None,
            camera_post_effect_active: false,
            post_effect_graphs: None,
            menu_background_blurriness: sanitize_menu_background_blurriness(
                VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS,
            ),
            hud_text_background_opacity: crate::hud::VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY,
            hud_background_for_chat_only: true,
            hide_gui: false,
            hud_screen_blur: false,
            transparency_combine_bind_group_layout,
            transparency_combine_pipeline,
            transparency_hand_pipeline,
            transparency_blit_bind_group_layout,
            transparency_blit_pipeline,
            targets,
            pipelines,
            pack,
            core_shader_generation: None,
            terrain_texture_filtering,
            // Vanilla `maxAnisotropyBit` default (`IntRange(1, 3)`, 2 → 4x);
            // consumed by the terrain sampler only under `Anisotropic`.
            terrain_max_anisotropy_bit: 2,
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
            frame_world_border_vertices: FrameDataBuffer::vertex(
                "bbb-client-world-border-frame-vertices",
            ),
            frame_world_border_indices: FrameDataBuffer::index(
                "bbb-client-world-border-frame-indices",
            ),
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
            hud_geometry: crate::hud::HudFrameGeometry::default(),
            hud_generation: 1,
            terrain_finish: None,
            lightmap,
            lightmap_dirty: true,
            lightmap_bind_group_layout,
            lightmap_hud_bind_group,
            _lightmap_hud_sampler: lightmap_hud_sampler,
            lightmap_sample_bind_group_layout,
            entity_outline_bind_group_layout,
            cloud_bind_group_layout,
            cloud_bind_group,
            cloud_quad_indices: None,
            hud_bind_group_layout,
            vignette_bind_group_layout,
            screen_effect_quads: Vec::new(),
            screen_effect_projection: None,
            screen_effect_vertices: FrameDataBuffer::vertex("bbb-screen-effect-vertices"),
            screen_effect_batches: Vec::new(),
            screen_effect_underwater: None,
            hud_white_pixel,
            #[cfg(test)]
            test_gui_sprite_images: Vec::new(),
            terrain_bind_group_layout,
            camera_buffers,
            pending_camera_uniforms: std::cell::Cell::new(None),
            pending_cloud_uniform: std::cell::Cell::new(None),
            hud_hotbar_block_item_models: Vec::new(),
            hud_offhand_slot: None,
            hud_offhand_block_item_model: None,
            hud_hotbar_attack_indicator: crate::hud::HudHotbarAttackIndicator::None,
            terrain: TerrainResidency::default(),
            frame_uploads,
            terrain_uploads,
            camera_pose: None,
            camera_hurt_effect: None,
            camera_view_bob_effect: None,
            camera_spinning_effect: None,
            camera_fov_degrees: 70.0,
            camera_hud_fov_degrees: 70.0,
            lightmap_environment: LightmapEnvironment::default(),
            fog_environment: FogEnvironment::default(),
            sky,
            cloud_environment: CloudEnvironment::default(),
            cloud_frame: CloudFrame::default(),
            cloud_shape: CloudShape::default(),
            clouds_enabled: true,
            block_destroy_overlays: BlockDestroyOverlaysGpu::new(),
            block_entity_crumbling_index_count: 0,
            entity_dynamic_player_skin_atlas: None,
            entity_dynamic_player_skin_images: Vec::new(),
            entity_dynamic_player_texture_atlas: None,
            entity_dynamic_player_texture_images: Vec::new(),
            dynamic_entity_texture_generation: None,
            entity_meshes_staged_unsubmitted: false,
            entity_model_instances: Vec::new(),
            entity_flame_surfaces: Vec::new(),
            entity_shadow_surfaces: Vec::new(),
            first_person_player_arms: Vec::new(),
            weather_render_state: WeatherRenderState::default(),
            world_border_render_state: WorldBorderRenderState::default(),
            item_entity_billboards: Vec::new(),
            #[cfg(test)]
            item_frame_map_surfaces: Vec::new(),
            #[cfg(test)]
            first_person_map_surfaces: Vec::new(),
            #[cfg(test)]
            first_person_map_background_surfaces: Vec::new(),
            #[cfg(test)]
            item_frame_map_decoration_surfaces: Vec::new(),
            #[cfg(test)]
            first_person_map_decoration_surfaces: Vec::new(),
            sign_text_surfaces: Vec::new(),
            visible_sign_text_positions: Vec::new(),
            sign_text_outline_context: SignTextOutlineContext::default(),
            display_text_surfaces: Vec::new(),
            entity_name_tag_surfaces: Vec::new(),
            block_selection: BlockSelectionGpu::new(),
            chunk_border_outline: SelectionOutlineGpu::new(),
            entity_scene_outline: SelectionOutlineGpu::new(),
            debug_filled_boxes: Vec::new(),
            debug_filled_rects: Vec::new(),
            debug_text_billboards: Vec::new(),
            debug_lines: Vec::new(),
            game_test_highlight_boxes: Vec::new(),
            game_test_highlight_text_billboards: Vec::new(),
            hud_camera_overlays: Vec::new(),
            hud_portal_overlay: None,
            hud_confusion_overlay: None,
            hud_crosshair_visible: true,
            hud_crosshair_attack_indicator: HudCrosshairAttackIndicator::None,
            hud_hotbar_item_icons: std::array::from_fn(|_| None),
            hud_now_playing_toast: None,
            hud_toasts: Vec::new(),
            hud_screen_present: false,
            hud_ingame_ui_present: false,
            hud_inventory_screen: None,
            hud_inventory_bundle_tooltip: None,
            hud_sign_editor_screen: None,
            hud_command_block_editor_screen: None,
            hud_structure_block_editor_screen: None,
            hud_jigsaw_block_editor_screen: None,
            hud_test_block_editor_screen: None,
            hud_test_instance_block_editor_screen: None,
            hud_pause_screen: None,
            hud_social_interactions_screen: None,
            hud_stats_screen: None,
            hud_game_rules_screen: None,
            hud_command_confirmation: None,
            hud_death_screen: None,
            hud_disconnected_screen: None,
            hud_level_loading_screen: None,
            hud_game_event_screen: None,
            hud_book_screen: None,
            hud_debug_options_screen: None,
            hud_sound_subtitles: crate::hud::HudSoundSubtitles::default(),
            hud_entity_preview_pip_targets: Vec::new(),
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
            hud_chat_options: crate::hud::HudChatOptions::default(),
            hud_chat_context: HudChatContext::default(),
            hud_chat_appended_lines: None,
            hud_chat_restrictions_screen: None,
            hud_command_suggestions: None,
            hud_debug_overlay: None,
            hud_boss_bars: Vec::new(),
            data_driven_rigs: Arc::from([]),
            dynamic_lights_mode: DynamicLightsMode::Off,
            dynamic_lights: Vec::new(),
            lod_terrain: crate::scene::lod_terrain::LodTerrainGpu::default(),
            lod_fog_end: 0.0,
            particle_frame: Default::default(),
            resource_reload_checkpoint: None,
        })
    }

    fn prepare_resize(
        &self,
        size: PhysicalSize<u32>,
        mut checkpoint: impl FnMut(RendererResizePrepareStage) -> Result<()>,
    ) -> Result<RendererResizePlan> {
        let plan = crate::post_effect_graph::capture_gpu_errors(&self.device, || {
            let post_effect_graphs = self
                .post_effect_graphs
                .as_ref()
                .map(|graphs| graphs.prepare_resize_in_scope(&self.device, size.width, size.height))
                .transpose()?;
            checkpoint(RendererResizePrepareStage::PostEffects)?;

            Ok(RendererResizePlan {
                size,
                post_effect_graphs,
                targets: self.targets.new_like(
                    &self.device,
                    self.config.format,
                    size.width,
                    size.height,
                    &self.frame_target_layouts(),
                ),
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
            graphs.apply_resize_with_retire(&self.queue, resize);
        }
        self.size = plan.size;
        self.config = config;
        let old_targets = std::mem::replace(&mut self.targets, plan.targets);
        old_targets.retire(&self.queue);
        self.pipelines
            .hud_gui
            .update_gui_extent(&self.queue, self.gui_projection_extent());
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

    /// `Window.updateVsync`: renegotiates the surface
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

    pub fn set_camera_post_effect(&mut self, id: Option<&str>, active: bool) {
        if self.current_post_effect_id.as_deref() == id && self.camera_post_effect_active == active
        {
            return;
        }
        self.current_post_effect_id = id.map(str::to_owned);
        self.camera_post_effect_active = active;
    }

    pub fn current_post_effect_id(&self) -> Option<&str> {
        self.current_post_effect_id.as_deref()
    }

    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    pub fn set_camera_pose(&mut self, pose: Option<CameraPose>) {
        if self.camera_pose == pose {
            return;
        }
        let previous_sort_position = self.camera_sort_position();
        self.camera_pose = pose;
        self.resort_translucent_terrain_for_camera();
        if self.pack.entity_model_texture_atlas.is_some()
            && !self.entity_model_instances.is_empty()
            && self.entity_model_meshes_have_camera_billboards()
        {
            self.refresh_entity_model_meshes();
        } else if previous_sort_position != self.camera_sort_position() {
            self.resort_entity_model_draws();
        }
        self.update_camera();
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
            self.terrain_max_anisotropy(),
        )?;
        self.set_terrain_atlas(terrain_atlas);
        self.counters.atlas_pages = 1;
        self.counters.atlas_reallocations += 1;
        self.counters.atlas_width = width;
        self.counters.atlas_height = height;
        self.invalidate_item_model_atlas(crate::item_models::ItemModelAtlasKind::Blocks);
        Ok(())
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

    pub fn set_gui_viewport(&mut self, size: PhysicalSize<u32>, gui_scale: u32) {
        let gui_scale = gui_scale.max(1);
        if self.gui_surface_size == size && self.gui_scale == gui_scale {
            return;
        }
        self.gui_surface_size = size;
        self.gui_scale = gui_scale;
        self.counters.gui_width = size.width;
        self.counters.gui_height = size.height;
        self.pipelines
            .hud_gui
            .update_gui_extent(&self.queue, self.gui_projection_extent());
        self.update_camera();
    }

    pub(crate) fn gui_projection_extent(&self) -> [f32; 2] {
        let scale = self.gui_scale as f32;
        [
            self.config.width.max(1) as f32 / scale,
            self.config.height.max(1) as f32 / scale,
        ]
    }

    pub fn gui_surface_size(&self) -> PhysicalSize<u32> {
        self.gui_surface_size
    }

    pub(crate) fn update_camera(&self) {
        self.write_camera_uniforms(self.camera_uniforms(), None);
    }

    fn camera_uniforms(&self) -> [CameraUniform; 3] {
        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        let glint_elapsed_millis = self.started_at.clock_elapsed().as_secs_f64() * 1000.0;
        let z_far = (self.lod_fog_end + 64.0).max(2048.0);
        let cameras = if let Some(pose) = self.camera_pose {
            [
                CameraUniform::from_pose_with_camera_effects_and_far(
                    pose,
                    aspect,
                    self.camera_fov_degrees,
                    self.camera_hurt_effect,
                    self.camera_view_bob_effect,
                    self.camera_spinning_effect,
                    z_far,
                ),
                CameraUniform::from_hud_pose_with_view_bob(
                    pose,
                    aspect,
                    self.camera_hud_fov_degrees,
                    self.camera_hurt_effect,
                    self.camera_view_bob_effect,
                ),
            ]
        } else {
            let camera = self
                .scene_bounds()
                .map(|bounds| CameraUniform::from_bounds(bounds, aspect))
                .unwrap_or_else(CameraUniform::identity);
            [camera; 2]
        };
        let camera_world_pos = self
            .camera_pose
            .map(|pose| {
                [
                    pose.position[0] as f64,
                    (pose.position[1] + pose.eye_height) as f64,
                    pose.position[2] as f64,
                ]
            })
            .unwrap_or([0.0; 3]);
        let [uniform, hud3d] = cameras.map(|camera| {
            camera
                .with_lightmap_environment(self.lightmap_environment)
                .with_fog_environment(self.fog_environment)
                .with_lod_fog_end(self.lod_fog_end)
                .with_glint_texture_time(glint_elapsed_millis, self.glint_speed, self.glint_alpha)
                .with_shader_game_time_ticks(self.shader_game_time_ticks)
                .with_viewport_size(self.config.width as f32, self.config.height as f32)
                .with_dynamic_lights(
                    self.dynamic_lights_mode,
                    camera_world_pos,
                    &self.dynamic_lights,
                )
        });

        // GuiRenderer keeps projection extent separate from ceil layout.
        let [width, height] = self.gui_projection_extent();
        let gui = CameraUniform::gui_ortho(width, height)
            .with_glint_texture_time(glint_elapsed_millis, self.glint_speed, self.glint_alpha)
            .with_shader_game_time_ticks(self.shader_game_time_ticks)
            .with_viewport_size(self.config.width as f32, self.config.height as f32);
        [uniform, hud3d, gui]
    }

    fn write_camera_uniforms(&self, uniforms: [CameraUniform; 3], cloud: Option<CloudUniform>) {
        self.pending_camera_uniforms.set(Some(uniforms));
        if let Some(cloud) = cloud {
            self.pending_cloud_uniform.set(Some(cloud));
        }
        #[cfg(test)]
        self.camera_uniform_writes
            .set(self.camera_uniform_writes.get().saturating_add(1));
    }

    pub(super) fn stage_pending_camera_uniforms(&mut self, encoder: &mut wgpu::CommandEncoder) {
        match (
            self.pending_camera_uniforms.get(),
            self.pending_cloud_uniform.get(),
        ) {
            (Some(uniforms), cloud) => {
                self.camera_buffers
                    .stage(encoder, self.frame_uploads.belt(), uniforms, cloud);
            }
            (None, Some(cloud)) => {
                self.camera_buffers
                    .stage_cloud(encoder, self.frame_uploads.belt(), cloud);
            }
            (None, None) => {}
        }
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

    pub fn set_cloud_frame(&mut self, environment: CloudEnvironment, frame: CloudFrame) {
        if self.replace_cloud_frame_inputs(environment, frame) && self.has_cloud_draws() {
            self.write_cloud_uniform();
        }
    }

    fn replace_cloud_frame_inputs(
        &mut self,
        environment: CloudEnvironment,
        frame: CloudFrame,
    ) -> bool {
        let environment = environment.sanitized();
        let frame = frame.sanitized();
        if self.cloud_environment == environment && self.cloud_frame == frame {
            return false;
        }
        let was_visible = self.cloud_environment.is_visible();
        self.cloud_environment = environment;
        self.cloud_frame = frame;
        if !environment.is_visible() {
            self.pack.clouds = None;
            return false;
        }
        if !was_visible
            || cloud_mesh_key(
                environment,
                self.pack.cloud_texture.as_ref(),
                frame,
                self.cloud_shape,
            ) != self.pack.cloud_mesh_key
        {
            self.rebuild_cloud_geometry();
        }
        true
    }

    pub fn cloud_shape(&self) -> Option<CloudShape> {
        self.clouds_enabled.then_some(self.cloud_shape)
    }

    /// Vanilla `Options.cloudStatus`: `OFF` skips the cloud pass, `FAST`
    /// builds the flat mesh, `FANCY` the boxed one.
    pub fn set_cloud_status(&mut self, enabled: bool, shape: CloudShape) {
        let was_visible = self.has_cloud_draws();
        self.clouds_enabled = enabled;
        if shape != self.cloud_shape {
            self.set_cloud_shape(shape);
        } else if !was_visible && self.has_cloud_draws() {
            self.write_cloud_uniform();
        }
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

#[cfg(test)]
mod tests {
    use super::{
        choose_format, choose_present_mode, resolve_fabulous_terrain_texture_filtering,
        LevelFrameRoute, Renderer, RendererCameraFrame, RendererInputLatencyStamp,
        RendererInputLatencyState, RendererResizePrepareStage, TerrainTextureFiltering,
    };
    use crate::camera::{CameraPose, LightmapEnvironment};
    use std::time::{Duration, Instant};
    use winit::dpi::PhysicalSize;

    mod borrowed_to_window;
    mod terrain;

    fn resize_target_sizes(renderer: &Renderer) -> Vec<wgpu::Extent3d> {
        let mut sizes = vec![
            renderer.targets.main._texture.size(),
            renderer.targets.depth._texture.size(),
        ];
        if let Some(improved) = renderer.targets.improved.as_ref() {
            sizes.extend([
                improved.translucent._texture.size(),
                improved.item_entity._texture.size(),
                improved.particle._texture.size(),
                improved.weather._texture.size(),
                improved.transparency_final._texture.size(),
                improved.cloud._texture.size(),
            ]);
        }
        if let Some(outline) = renderer.targets.entity_outline.as_ref() {
            sizes.push(outline._texture.size());
        }
        sizes
    }

    #[test]
    fn offscreen_frame_targets_allocate_improved_and_outline_lazily_and_preserve_on_resize() {
        let Some(mut renderer) = Renderer::new_offscreen(64, 64) else {
            return;
        };
        assert!(renderer.targets.improved.is_none());
        assert!(renderer.targets.entity_outline.is_none());
        assert_eq!(resize_target_sizes(&renderer).len(), 2);

        // Resizing while inactive does not allocate optional target sets.
        renderer.resize(PhysicalSize::new(96, 80)).unwrap();
        assert!(renderer.targets.improved.is_none());
        assert!(renderer.targets.entity_outline.is_none());
        assert_eq!(resize_target_sizes(&renderer).len(), 2);

        // Activating improved transparency and outline allocates them and keeps them across resize.
        renderer.ensure_frame_targets(true, true);
        assert!(renderer.targets.improved.is_some());
        assert!(renderer.targets.entity_outline.is_some());
        assert_eq!(resize_target_sizes(&renderer).len(), 9);
        renderer.resize(PhysicalSize::new(128, 72)).unwrap();
        assert_eq!(resize_target_sizes(&renderer).len(), 9);
        assert!(resize_target_sizes(&renderer)
            .iter()
            .all(|size| size.width == 128 && size.height == 72));
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
    fn fabulous_filtering_defaults_uniformly_to_anisotropic_unless_overridden() {
        assert_eq!(
            resolve_fabulous_terrain_texture_filtering(TerrainTextureFiltering::None, true),
            TerrainTextureFiltering::Anisotropic
        );
        assert_eq!(
            resolve_fabulous_terrain_texture_filtering(TerrainTextureFiltering::Rgss, true),
            TerrainTextureFiltering::Anisotropic
        );
        assert_eq!(
            resolve_fabulous_terrain_texture_filtering(TerrainTextureFiltering::None, false),
            TerrainTextureFiltering::None
        );
        assert_eq!(
            resolve_fabulous_terrain_texture_filtering(TerrainTextureFiltering::Rgss, false),
            TerrainTextureFiltering::Rgss
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
        assert_eq!(renderer.counters().gui_width, 1920);
        assert_eq!(renderer.counters().gui_height, 1080);

        // 1920x1080 at auto scale 4 publishes 480x270 to the renderer.
        renderer.set_gui_viewport(PhysicalSize::new(480, 270), 4);
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1920, 1080));
        assert_eq!(renderer.counters().gui_width, 480);
        assert_eq!(renderer.counters().gui_height, 270);

        // A zero-size resize only marks minimized; both last usable sizes are
        // retained until a nonzero resize replaces the physical surface.
        renderer.resize(PhysicalSize::new(0, 0)).unwrap();
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1920, 1080));
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));

        renderer.resize(PhysicalSize::new(1280, 720)).unwrap();
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1280, 720));
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));
        let counters = renderer.counters();
        assert_eq!([counters.width, counters.height], [1280, 720]);
        assert_eq!([counters.gui_width, counters.gui_height], [480, 270]);
    }

    #[test]
    fn set_gui_viewport_refreshes_dependent_camera_state_and_no_ops_when_unchanged() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(1920, 1080) else {
            return;
        };
        let baseline = renderer.camera_uniform_writes.get();

        // A changed logical GUI size publishes through `update_camera`, which
        // rewrites the GUI-item ortho buffer from the new size; the world
        // camera keeps reading the physical config.
        renderer.set_gui_viewport(PhysicalSize::new(480, 270), 4);
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(480, 270));
        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 1);
        assert_eq!(renderer.surface_size(), PhysicalSize::new(1920, 1080));

        // An unchanged size is a true no-op, not a redundant uniform rewrite.
        renderer.set_gui_viewport(PhysicalSize::new(480, 270), 4);
        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 1);

        // A second change refreshes again; 1280x720 at scale 3 projects
        // 427x240 (ceiling division).
        renderer.set_gui_viewport(PhysicalSize::new(427, 240), 3);
        assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(427, 240));
        assert_eq!(renderer.camera_uniform_writes.get(), baseline + 2);
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
}
