use anyhow::{anyhow, Result};
use bbb_control::RendererCounters;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    block_destroy::{
        create_block_destroy_overlays_gpu, create_block_destroy_pipeline, BlockDestroyOverlay,
        BlockDestroyOverlaysGpu,
    },
    camera::{CameraPose, CameraUniform, ClearColor, TerrainBounds},
    entity_models::{
        create_entity_model_pipeline, create_entity_model_textured_pipeline, EntityModelMeshGpu,
        EntityModelTextureAtlasGpu, EntityModelTexturedMeshGpu,
    },
    gpu::{
        create_camera_buffer, create_depth_target, create_terrain_atlas_gpu,
        create_terrain_atlas_mips_gpu, create_terrain_bind_group, create_terrain_bind_group_layout,
        create_terrain_pipeline, create_terrain_translucent_pipeline, write_terrain_atlas_gpu,
        write_terrain_atlas_mips_gpu, DepthTarget, TerrainAtlasGpu,
    },
    hud::{
        create_hud_bind_group_layout, create_hud_pipeline, create_hud_sprite_gpu, HudAsciiGlyph,
        HudDigitGlyph, HudInventoryScreen, HudItemIcon, HudSpriteGpu, HUD_ASCII_GLYPH_COUNT,
        HUD_HOTBAR_SLOTS,
    },
    item_entities::{create_item_entity_pipeline, ItemEntityAtlasGpu, ItemEntityBillboard},
    particles::{create_particle_pipeline, ParticleAtlasGpu, ParticleRuntimeState},
    selection::{
        create_selection_outline_gpu, create_selection_pipeline, SelectionOutline,
        SelectionOutlineGpu,
    },
    terrain,
};

pub struct Renderer {
    pub(super) surface: wgpu::Surface<'static>,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) size: PhysicalSize<u32>,
    pub(super) clear: ClearColor,
    pub(super) counters: RendererCounters,
    pub(super) depth: DepthTarget,
    pub(super) terrain_pipeline: wgpu::RenderPipeline,
    pub(super) terrain_translucent_pipeline: wgpu::RenderPipeline,
    pub(super) block_destroy_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_pipeline: wgpu::RenderPipeline,
    pub(super) entity_model_textured_pipeline: wgpu::RenderPipeline,
    pub(super) particle_pipeline: wgpu::RenderPipeline,
    pub(super) item_entity_pipeline: wgpu::RenderPipeline,
    pub(super) selection_pipeline: wgpu::RenderPipeline,
    pub(super) hud_pipeline: wgpu::RenderPipeline,
    pub(super) hud_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) hud_white_pixel: HudSpriteGpu,
    pub(super) terrain_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) camera_buffer: wgpu::Buffer,
    pub(super) terrain_atlas: TerrainAtlasGpu,
    pub(super) terrain_bind_group: wgpu::BindGroup,
    pub(super) terrain_opaque: Vec<ResidentTerrainMesh>,
    pub(super) terrain_cutout: Vec<ResidentTerrainMesh>,
    pub(super) terrain_translucent: Vec<ResidentTerrainMesh>,
    pub(super) terrain_source_sections: usize,
    pub(super) terrain_bounds: Option<TerrainBounds>,
    pub(super) entity_model_bounds: Option<TerrainBounds>,
    pub(super) camera_pose: Option<CameraPose>,
    pub(super) block_destroy_overlays: Option<BlockDestroyOverlaysGpu>,
    pub(super) entity_model_mesh: Option<EntityModelMeshGpu>,
    pub(super) entity_model_textured_mesh: Option<EntityModelTexturedMeshGpu>,
    pub(super) entity_model_texture_atlas: Option<EntityModelTextureAtlasGpu>,
    pub(super) entity_model_instances: Vec<crate::EntityModelInstance>,
    pub(super) particle_atlas: Option<ParticleAtlasGpu>,
    pub(super) item_entity_atlas: Option<ItemEntityAtlasGpu>,
    pub(super) item_entity_billboards: Vec<ItemEntityBillboard>,
    pub(super) selection_outline: Option<SelectionOutlineGpu>,
    pub(super) entity_scene_outline: Option<SelectionOutlineGpu>,
    pub(super) entity_target_outline: Option<SelectionOutlineGpu>,
    pub(super) hud_crosshair: Option<HudSpriteGpu>,
    pub(super) hud_hotbar: Option<HudSpriteGpu>,
    pub(super) hud_hotbar_selection: Option<HudSpriteGpu>,
    pub(super) hud_item_atlas: Option<HudSpriteGpu>,
    pub(super) hud_digit_atlas: Option<HudSpriteGpu>,
    pub(super) hud_digit_glyphs: [HudDigitGlyph; 10],
    pub(super) hud_ascii_atlas: Option<HudSpriteGpu>,
    pub(super) hud_ascii_glyphs: [HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
    pub(super) hud_hotbar_item_icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS],
    pub(super) hud_inventory_background: Option<HudSpriteGpu>,
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
    pub(super) hud_experience_background: Option<HudSpriteGpu>,
    pub(super) hud_experience_progress: Option<HudSpriteGpu>,
    pub(super) hud_heart_container: Option<HudSpriteGpu>,
    pub(super) hud_heart_full: Option<HudSpriteGpu>,
    pub(super) hud_heart_half: Option<HudSpriteGpu>,
    pub(super) hud_food_empty: Option<HudSpriteGpu>,
    pub(super) hud_food_full: Option<HudSpriteGpu>,
    pub(super) hud_food_half: Option<HudSpriteGpu>,
    pub(super) hud_code_of_conduct_overlay: Option<HudSpriteGpu>,
    pub(super) hud_health: Option<f32>,
    pub(super) hud_food: Option<i32>,
    pub(super) hud_experience_progress_value: Option<f32>,
    pub(super) hud_selected_slot: u8,
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
        let depth = create_depth_target(&device, config.width, config.height);
        let terrain_bind_group_layout = create_terrain_bind_group_layout(&device);
        let hud_bind_group_layout = create_hud_bind_group_layout(&device);
        let camera_buffer = create_camera_buffer(&device);
        let terrain_atlas = create_terrain_atlas_gpu(&device, &queue, 1, 1, &[255, 255, 255, 255])?;
        let terrain_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &camera_buffer,
            &terrain_atlas,
        );
        let terrain_pipeline = create_terrain_pipeline(&device, format, &terrain_bind_group_layout);
        let terrain_translucent_pipeline =
            create_terrain_translucent_pipeline(&device, format, &terrain_bind_group_layout);
        let block_destroy_pipeline =
            create_block_destroy_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_pipeline =
            create_entity_model_pipeline(&device, format, &terrain_bind_group_layout);
        let entity_model_textured_pipeline =
            create_entity_model_textured_pipeline(&device, format, &terrain_bind_group_layout);
        let particle_pipeline =
            create_particle_pipeline(&device, format, &terrain_bind_group_layout);
        let item_entity_pipeline =
            create_item_entity_pipeline(&device, format, &terrain_bind_group_layout);
        let selection_pipeline =
            create_selection_pipeline(&device, format, &terrain_bind_group_layout);
        let hud_pipeline = create_hud_pipeline(&device, format, &hud_bind_group_layout);
        let hud_white_pixel = create_hud_sprite_gpu(
            &device,
            &queue,
            &hud_bind_group_layout,
            1,
            1,
            &[255, 255, 255, 255],
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
            depth,
            terrain_pipeline,
            terrain_translucent_pipeline,
            block_destroy_pipeline,
            entity_model_pipeline,
            entity_model_textured_pipeline,
            particle_pipeline,
            item_entity_pipeline,
            selection_pipeline,
            hud_pipeline,
            hud_bind_group_layout,
            hud_white_pixel,
            terrain_bind_group_layout,
            camera_buffer,
            terrain_atlas,
            terrain_bind_group,
            terrain_opaque: Vec::new(),
            terrain_cutout: Vec::new(),
            terrain_translucent: Vec::new(),
            terrain_source_sections: 0,
            terrain_bounds: None,
            entity_model_bounds: None,
            camera_pose: None,
            block_destroy_overlays: None,
            entity_model_mesh: None,
            entity_model_textured_mesh: None,
            entity_model_texture_atlas: None,
            entity_model_instances: Vec::new(),
            particle_atlas: None,
            item_entity_atlas: None,
            item_entity_billboards: Vec::new(),
            selection_outline: None,
            entity_scene_outline: None,
            entity_target_outline: None,
            hud_crosshair: None,
            hud_hotbar: None,
            hud_hotbar_selection: None,
            hud_item_atlas: None,
            hud_digit_atlas: None,
            hud_digit_glyphs: [HudDigitGlyph::default(); 10],
            hud_ascii_atlas: None,
            hud_ascii_glyphs: [HudAsciiGlyph::default(); HUD_ASCII_GLYPH_COUNT],
            hud_hotbar_item_icons: std::array::from_fn(|_| None),
            hud_inventory_background: None,
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
            hud_experience_background: None,
            hud_experience_progress: None,
            hud_heart_container: None,
            hud_heart_full: None,
            hud_heart_half: None,
            hud_food_empty: None,
            hud_food_full: None,
            hud_food_half: None,
            hud_code_of_conduct_overlay: None,
            hud_health: None,
            hud_food: None,
            hud_experience_progress_value: None,
            hud_selected_slot: 0,
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
        self.depth = create_depth_target(&self.device, self.config.width, self.config.height);
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
        })
    }

    pub(crate) fn update_camera(&self) {
        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        let uniform = if let Some(pose) = self.camera_pose {
            CameraUniform::from_pose(pose, aspect)
        } else {
            self.scene_bounds()
                .map(|bounds| CameraUniform::from_bounds(bounds, aspect))
                .unwrap_or_else(CameraUniform::identity)
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
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
