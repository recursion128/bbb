//! Renderer owners derived from one resource generation. Startup and every reload begin from
//! the same fresh value; a reload checkpoint keeps the previous value whole.

use super::*;
use crate::texture_sampling::{
    create_camera_texture_bind_group, create_texture_bind_group, standalone_sampler_descriptor,
};
use bbb_render_types::StandaloneTextureSampling;

/// The device, layouts and camera buffers pack-derived owners are built against.
pub(crate) struct PackContext<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) format: wgpu::TextureFormat,
    pub(super) terrain_layout: &'a wgpu::BindGroupLayout,
    pub(super) lightmap_sample_layout: &'a wgpu::BindGroupLayout,
    pub(super) hud_layout: &'a wgpu::BindGroupLayout,
    pub(super) camera: wgpu::BufferBinding<'a>,
    pub(super) hud3d_camera: wgpu::BufferBinding<'a>,
    pub(super) gui_item_camera: wgpu::BufferBinding<'a>,
}

pub(crate) struct PackResources {
    pub(crate) terrain_atlas: TerrainAtlasGpu,
    pub(crate) terrain_atlas_animation: Option<TerrainAtlasAnimationGpu>,
    pub(crate) terrain_bind_group: wgpu::BindGroup,
    pub(crate) hud3d_terrain_bind_group: wgpu::BindGroup,
    pub(crate) hud_terrain_atlas_bind_group: wgpu::BindGroup,
    /// The GUI item pass samples the blocks atlas through its own ortho camera.
    pub(crate) gui_item_bind_group: wgpu::BindGroup,
    pub(crate) particle_atlas: Option<ParticleAtlasGpu>,
    pub(crate) particle_sources: crate::particles::ParticleQuadSources,
    pub(crate) particle_light: crate::particles::ParticleLightSamples,
    pub(crate) terrain_particle_sprite_uvs: Vec<ParticleSpriteUv>,
    pub(crate) item_particle_sprite_uvs: Vec<ParticleSpriteUv>,
    pub(crate) item_entity_atlas: Option<ItemEntityAtlasGpu>,
    pub(crate) item_glint_texture: Option<GlintTextureGpu>,
    pub(crate) armor_glint_texture: Option<GlintTextureGpu>,
    pub(crate) item_model_residency: ItemModelResidency,
    pub(crate) entity_name_tags: crate::entity_models::EntityNameTagRenderState,
    pub(crate) entity_flames: crate::entity_models::EntityFeatureMesh,
    pub(crate) entity_shadows: crate::entity_models::EntityFeatureMesh,
    pub(crate) display_text: crate::item_render::item_models::display_text::DisplayTextRenderState,
    pub(crate) sign_text: crate::item_render::item_models::sign_text::SignTextRenderState,
    pub(crate) item_frame_map_text_font_atlas: Option<ItemFrameMapTextFontAtlasGpu>,
    pub(crate) first_person_map_background_atlas: Option<FirstPersonMapBackgroundAtlasGpu>,
    pub(crate) item_frame_map_atlas: Option<ItemFrameMapAtlasGpu>,
    pub(crate) item_frame_map_decoration_atlas: Option<ItemFrameMapDecorationAtlasGpu>,
    pub(crate) map_surface_residency: MapSurfaceResidency,
    pub(crate) entity_model_texture_atlas: Option<EntityModelTextureAtlasGpu>,
    pub(crate) entity_meshes: EntityMeshes,
    pub(crate) entity_geometry: crate::entity_models::EntityGeometryResidency,
    pub(crate) entity_extract: crate::entity_models::EntityExtractionCache,
    pub(crate) entity_shadow_texture: Option<EntityShadowTextureGpu>,
    pub(crate) hud_win_portal_mesh: Option<EntityModelMeshGpu>,
    pub(crate) first_person_player_arm_mesh: Option<EntityModelMeshGpu>,
    pub(crate) first_person_dynamic_player_arm_mesh: Option<EntityModelMeshGpu>,
    pub(crate) cloud_texture: Option<CloudTextureData>,
    pub(crate) clouds: Option<CloudGpu>,
    pub(super) cloud_mesh_key: Option<crate::clouds::CloudMeshKey>,
    pub(crate) weather_rain_texture: Option<WeatherTextureGpu>,
    pub(crate) weather_snow_texture: Option<WeatherTextureGpu>,
    pub(crate) world_border_texture: Option<WorldBorderTextureGpu>,
    pub(crate) gui_sprites: crate::hud::GuiSprites,
    pub(crate) gui_textures: HashMap<Box<str>, HudSpriteGpu>,
    pub(crate) hud_default_player_faces:
        HashMap<EntityDefaultPlayerSkin, crate::hud::HudPlayerFaceGpu>,
    pub(crate) hud_dynamic_player_faces: HashMap<u64, crate::hud::HudPlayerFaceGpu>,
    pub(crate) hud_digit_glyphs: [HudDigitGlyph; 10],
    pub(crate) hud_font_atlas: Option<HudSpriteGpu>,
    pub(crate) hud_font_dynamic_layout_generation: u64,
    pub(crate) hud_font_dynamic_upload_revision: u64,
    pub(crate) hud_font_glyphs: HudFontGlyphMap,
    pub(crate) hud_font_book: HudFontGlyphBook,
    /// Advance-grouped glyphs keep obfuscated text draws from rescanning the font.
    pub(crate) hud_obfuscated_glyph_pool: HudObfuscatedGlyphPool,
    pub(crate) hud_panorama: Option<crate::panorama::PanoramaGpu>,
    pub(crate) hud_locator_styles: BTreeMap<String, crate::hud::HudLocatorStyle>,
    pub(crate) hud_vignette_texture: Option<VignetteTextureGpu>,
    pub(crate) hud_win_credits_vignette: Option<VignetteTextureGpu>,
    pub(crate) end_sky_texture: Option<crate::sky::EndSkyTextureGpu>,
    pub(crate) celestial_atlas: Option<crate::sky::CelestialAtlasGpu>,
}

impl PackResources {
    /// The empty generation: a white 1×1 terrain atlas and no pack owners.
    pub(crate) fn new(
        cx: &PackContext<'_>,
        item_model_residency: ItemModelResidency,
        entity_geometry: crate::entity_models::EntityGeometryResidency,
    ) -> Result<Self> {
        let terrain_atlas =
            create_terrain_atlas_gpu(cx.device, cx.queue, 1, 1, &[255, 255, 255, 255])?;
        let [terrain_bind_group, hud3d_terrain_bind_group, hud_terrain_atlas_bind_group, gui_item_bind_group] =
            terrain_atlas_bind_groups(cx, &terrain_atlas);
        Ok(Self {
            terrain_atlas,
            terrain_atlas_animation: None,
            terrain_bind_group,
            hud3d_terrain_bind_group,
            hud_terrain_atlas_bind_group,
            gui_item_bind_group,
            particle_atlas: None,
            particle_sources: Default::default(),
            particle_light: Default::default(),
            terrain_particle_sprite_uvs: Vec::new(),
            item_particle_sprite_uvs: Vec::new(),
            item_entity_atlas: None,
            item_glint_texture: None,
            armor_glint_texture: None,
            item_model_residency,
            entity_name_tags: Default::default(),
            entity_flames: Default::default(),
            entity_shadows: Default::default(),
            display_text: Default::default(),
            sign_text: Default::default(),
            item_frame_map_text_font_atlas: None,
            first_person_map_background_atlas: None,
            item_frame_map_atlas: None,
            item_frame_map_decoration_atlas: None,
            map_surface_residency: MapSurfaceResidency::new(
                cx.device,
                cx.format,
                cx.terrain_layout,
                cx.lightmap_sample_layout,
            ),
            entity_model_texture_atlas: None,
            entity_meshes: EntityMeshes::default(),
            entity_geometry,
            entity_extract: Default::default(),
            entity_shadow_texture: None,
            hud_win_portal_mesh: None,
            first_person_player_arm_mesh: None,
            first_person_dynamic_player_arm_mesh: None,
            cloud_texture: None,
            clouds: None,
            cloud_mesh_key: None,
            weather_rain_texture: None,
            weather_snow_texture: None,
            world_border_texture: None,
            gui_sprites: Default::default(),
            gui_textures: HashMap::new(),
            hud_default_player_faces: HashMap::new(),
            hud_dynamic_player_faces: HashMap::new(),
            hud_digit_glyphs: [HudDigitGlyph::default(); 10],
            hud_font_atlas: None,
            hud_font_dynamic_layout_generation: 0,
            hud_font_dynamic_upload_revision: 0,
            hud_font_glyphs: HudFontGlyphMap::default(),
            hud_font_book: HudFontGlyphBook::default(),
            hud_obfuscated_glyph_pool: HudObfuscatedGlyphPool::default(),
            hud_panorama: None,
            hud_locator_styles: BTreeMap::new(),
            hud_vignette_texture: None,
            hud_win_credits_vignette: None,
            end_sky_texture: None,
            celestial_atlas: None,
        })
    }

    /// Defers explicit destruction of all pack-owned GPU textures and buffers on WebGPU until
    /// already-submitted queue work completes.
    pub(crate) fn retire(self, queue: &wgpu::Queue) {
        let mut buffers = Vec::new();
        let mut textures = vec![self.terrain_atlas.texture];
        if let Some(anim) = self.terrain_atlas_animation {
            anim.collect_retire(&mut buffers, &mut textures);
        }
        if let Some(particles) = self.particle_atlas {
            textures.push(particles.texture);
        }
        if let Some(items) = self.item_entity_atlas {
            if let Ok(gpu) = std::sync::Arc::try_unwrap(items.hud.gpu) {
                textures.push(gpu.texture);
            }
        }
        if let Some(glint) = self.item_glint_texture {
            textures.push(glint.into_texture());
        }
        if let Some(glint) = self.armor_glint_texture {
            textures.push(glint.into_texture());
        }
        self.item_model_residency.collect_retire(&mut buffers);
        for world_buf in self.entity_name_tags.buffers {
            world_buf.collect_retire(&mut buffers);
        }
        self.entity_flames.buffers.collect_retire(&mut buffers);
        self.entity_shadows.buffers.collect_retire(&mut buffers);
        for world_buf in self.display_text.buffers {
            world_buf.collect_retire(&mut buffers);
        }
        for world_buf in self.sign_text.buffers {
            world_buf.collect_retire(&mut buffers);
        }
        if let Some(font) = self.item_frame_map_text_font_atlas {
            textures.push(font.into_texture());
        }
        if let Some(bg) = self.first_person_map_background_atlas {
            textures.push(bg.into_texture());
        }
        if let Some(map_atlas) = self.item_frame_map_atlas {
            map_atlas.collect_retire(&mut textures);
        }
        if let Some(decorations) = self.item_frame_map_decoration_atlas {
            textures.push(decorations.into_texture());
        }
        self.map_surface_residency.collect_retire(&mut buffers);
        if let Some(entity_atlas) = self.entity_model_texture_atlas {
            textures.push(entity_atlas.into_texture());
        }
        for mesh in self.entity_meshes.lanes.into_iter().flatten() {
            mesh.collect_retire(&mut buffers);
        }
        self.entity_geometry.collect_retire(&mut buffers);
        if let Some(shadow) = self.entity_shadow_texture {
            textures.push(shadow.into_texture());
        }
        for mesh in [
            self.hud_win_portal_mesh,
            self.first_person_player_arm_mesh,
            self.first_person_dynamic_player_arm_mesh,
        ]
        .into_iter()
        .flatten()
        {
            mesh.collect_retire(&mut buffers);
        }
        if let Some(clouds) = self.clouds {
            buffers.push(clouds.face_buffer);
        }
        if let Some(rain) = self.weather_rain_texture {
            textures.push(rain.into_texture());
        }
        if let Some(snow) = self.weather_snow_texture {
            textures.push(snow.into_texture());
        }
        if let Some(border) = self.world_border_texture {
            textures.push(border.into_texture());
        }
        self.gui_sprites.collect_retire(&mut textures);
        for sprite in self.gui_textures.into_values() {
            if let Ok(gpu) = std::sync::Arc::try_unwrap(sprite.gpu) {
                textures.push(gpu.texture);
            }
        }
        for face in self
            .hud_default_player_faces
            .into_values()
            .chain(self.hud_dynamic_player_faces.into_values())
        {
            face.collect_retire(&mut textures);
        }
        if let Some(font) = self.hud_font_atlas {
            if let Ok(gpu) = std::sync::Arc::try_unwrap(font.gpu) {
                textures.push(gpu.texture);
            }
        }
        if let Some(panorama) = self.hud_panorama {
            panorama.collect_retire(&mut buffers, &mut textures);
        }
        if let Some(vignette) = self.hud_vignette_texture {
            vignette.collect_retire(&mut buffers, &mut textures);
        }
        if let Some(vignette) = self.hud_win_credits_vignette {
            vignette.collect_retire(&mut buffers, &mut textures);
        }
        if let Some(end_sky) = self.end_sky_texture {
            textures.push(end_sky._texture);
        }
        if let Some(celestial) = self.celestial_atlas {
            textures.push(celestial._texture);
        }
        crate::frame_buffers::retire_gpu_resources(queue, buffers, textures);
    }
}

/// World, HUD-3D, HUD and GUI-item bind groups over one terrain atlas.
fn terrain_atlas_bind_groups(
    cx: &PackContext<'_>,
    atlas: &TerrainAtlasGpu,
) -> [wgpu::BindGroup; 4] {
    let (view, sampler) = atlas.view_and_sampler();
    // TextureAtlas's HUD and screen-effect sampler is nearest, unlike chunk layers.
    let hud_sampler = cx.device.create_sampler(&standalone_sampler_descriptor(
        "bbb-hud-terrain-atlas-sampler",
        StandaloneTextureSampling::new(false, true),
    ));
    let terrain = |camera| {
        create_camera_texture_bind_group(
            cx.device,
            cx.terrain_layout,
            "bbb-terrain-bind-group",
            camera,
            view,
            sampler,
        )
    };
    [
        terrain(&cx.camera),
        terrain(&cx.hud3d_camera),
        create_texture_bind_group(
            cx.device,
            cx.hud_layout,
            "bbb-hud-terrain-atlas-bind-group",
            view,
            &hud_sampler,
        ),
        terrain(&cx.gui_item_camera),
    ]
}

impl Renderer {
    pub(crate) fn pack_context(&self) -> PackContext<'_> {
        PackContext {
            device: &self.device,
            queue: &self.queue,
            format: self.config.format,
            terrain_layout: &self.terrain_bind_group_layout,
            lightmap_sample_layout: &self.lightmap_sample_bind_group_layout,
            hud_layout: &self.hud_bind_group_layout,
            camera: self.camera_buffers.world(),
            hud3d_camera: self.camera_buffers.hud3d(),
            gui_item_camera: self.camera_buffers.gui(),
        }
    }

    pub(super) fn set_terrain_atlas(&mut self, atlas: TerrainAtlasGpu) {
        let bind_groups = terrain_atlas_bind_groups(&self.pack_context(), &atlas);
        let pack = &mut self.pack;
        [
            pack.terrain_bind_group,
            pack.hud3d_terrain_bind_group,
            pack.hud_terrain_atlas_bind_group,
            pack.gui_item_bind_group,
        ] = bind_groups;
        let previous = std::mem::replace(&mut pack.terrain_atlas, atlas);
        crate::frame_buffers::retire_gpu_textures(&self.queue, [previous.texture]);
    }
}
