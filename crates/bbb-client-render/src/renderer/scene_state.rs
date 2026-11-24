use super::*;

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

impl Renderer {
    /// Replaces the frame's vanilla `ScreenEffectRenderer` overlays, in the
    /// vanilla draw order (view-blocking block, then underwater, then fire).
    pub fn set_screen_effect_quads(&mut self, quads: Vec<crate::screen_effects::ScreenEffectQuad>) {
        if self.screen_effect_quads != quads {
            self.screen_effect_quads = quads;
            self.screen_effect_projection = None;
        }
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

    pub fn camera_post_effect_active(&self) -> bool {
        self.camera_post_effect_active
    }

    /// Sets vanilla's startup-only menu background blur option (`0..=10`).
    /// A value below one disables `Screen.extractBlurredBackground`'s stratum marker.
    pub fn set_menu_background_blurriness(&mut self, blurriness: u8) {
        let next = sanitize_menu_background_blurriness(blurriness);
        if self.menu_background_blurriness.to_bits() != next.to_bits() {
            self.menu_background_blurriness = next;
            self.mark_hud_dirty();
        }
    }

    /// Sets the startup-only equivalents of Vanilla's accessibility text
    /// background options. Chat always uses the configured opacity; other
    /// `textWithBackdrop` consumers use it only when chat-only mode is off.
    pub fn set_hud_text_background_options(
        &mut self,
        opacity: f32,
        background_for_chat_only: bool,
    ) {
        let next_opacity = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0)
        } else {
            crate::hud::VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY
        };
        if self.hud_text_background_opacity.to_bits() != next_opacity.to_bits()
            || self.hud_background_for_chat_only != background_for_chat_only
        {
            self.hud_text_background_opacity = next_opacity;
            self.hud_background_for_chat_only = background_for_chat_only;
            self.mark_hud_dirty();
        }
    }

    /// Marks whether the projected screen inherited vanilla's non-in-game background.
    pub fn set_hud_screen_blur(&mut self, active: bool) {
        if self.hud_screen_blur != active {
            self.hud_screen_blur = active;
            self.mark_hud_dirty();
        }
    }

    /// Replaces every camera-uniform input for one renderer frame and publishes
    /// the final projection once. Standalone setters remain available for
    /// initialization and event-driven updates outside the frame handoff.
    pub fn set_camera_frame(&mut self, frame: RendererCameraFrame) {
        self.publish_scene_frame(frame, false, None);
    }

    /// Replaces camera and entity inputs before rebuilding camera-dependent
    /// geometry, returning cleared storage for the next extraction.
    pub fn replace_scene_frame(
        &mut self,
        camera: RendererCameraFrame,
        instances: Vec<crate::EntityModelInstance>,
    ) -> Vec<crate::EntityModelInstance> {
        let (previous, entities_changed) = self.replace_entity_model_inputs(instances);
        self.publish_scene_frame(camera, entities_changed, None);
        previous
    }

    pub fn replace_scene_frame_with_clouds(
        &mut self,
        camera: RendererCameraFrame,
        instances: Vec<crate::EntityModelInstance>,
        environment: CloudEnvironment,
        clouds: CloudFrame,
    ) -> Vec<crate::EntityModelInstance> {
        let (previous, entities_changed) = self.replace_entity_model_inputs(instances);
        self.publish_scene_frame(camera, entities_changed, Some((environment, clouds)));
        previous
    }

    fn replace_camera_frame_inputs(&mut self, frame: RendererCameraFrame) -> (bool, bool) {
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
        self.sky.set_environment(
            &self.device,
            &mut self.frame_uploads,
            sky_environment,
            self.pack.celestial_atlas.as_ref(),
        );

        let previous_sort_position = self.camera_sort_position();
        let pose_changed = self.camera_pose != pose;
        self.camera_pose = pose;
        self.camera_hurt_effect = hurt_effect;
        self.camera_view_bob_effect = view_bob_effect;
        self.camera_spinning_effect = spinning_effect;
        self.camera_fov_degrees = sanitize_camera_fov_degrees(fov_degrees);
        self.camera_hud_fov_degrees = sanitize_camera_fov_degrees(hud_fov_degrees);
        self.shader_game_time_ticks = sanitize_shader_game_time_ticks(shader_game_time_ticks);
        (
            pose_changed,
            previous_sort_position != self.camera_sort_position(),
        )
    }

    fn publish_scene_frame(
        &mut self,
        camera: RendererCameraFrame,
        entities_changed: bool,
        clouds: Option<(CloudEnvironment, CloudFrame)>,
    ) {
        let clouds_were_visible = self.has_cloud_draws();
        let (pose_changed, sort_position_changed) = self.replace_camera_frame_inputs(camera);
        if pose_changed {
            self.resort_translucent_terrain_for_camera();
        }
        if entities_changed
            || (pose_changed
                && self.pack.entity_model_texture_atlas.is_some()
                && !self.entity_model_instances.is_empty()
                && self.entity_model_meshes_have_camera_billboards())
        {
            self.refresh_entity_model_meshes();
        } else if sort_position_changed {
            self.resort_entity_model_draws();
        }
        // Glint still advances while the simulation clock is paused.
        let uniforms = self.camera_uniforms();
        let changed = clouds.is_some_and(|(environment, frame)| {
            self.replace_cloud_frame_inputs(environment, frame)
        });
        let cloud = self.cloud_uniform_if_visible(changed || !clouds_were_visible);
        self.write_camera_uniforms(uniforms, cloud);
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
        self.block_selection
            .update(&self.device, &self.queue, state);
    }

    pub fn set_chunk_border_outline(&mut self, outline: Option<SelectionOutline>) {
        let chunk_border_boxes = outline.as_ref().map_or(0, |outline| {
            outline.boxes.len() + outline.colored_boxes.len()
        });
        self.counters.chunk_border_boxes = chunk_border_boxes;
        self.chunk_border_outline
            .update(&self.device, &self.queue, outline);
    }

    pub fn set_entity_scene_outline(&mut self, outline: Option<SelectionOutline>) {
        let entity_scene_boxes = outline.as_ref().map_or(0, |outline| {
            outline.boxes.len() + outline.colored_boxes.len()
        });
        self.counters.entity_scene_boxes = entity_scene_boxes;
        self.entity_scene_outline
            .update(&self.device, &self.queue, outline);
    }

    pub fn set_block_destroy_overlays(&mut self, overlays: Vec<BlockDestroyOverlay>) {
        self.block_destroy_overlays
            .update(&self.device, &self.queue, overlays);
    }

    pub fn set_lightmap_brightness_factor(&mut self, factor: f32) {
        let mut environment = self.lightmap_environment;
        environment.brightness_factor = sanitize_lightmap_brightness_factor(factor);
        self.set_lightmap_environment(environment);
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
        let clouds_were_visible = self.has_cloud_draws();
        self.replace_fog_environment(environment);
        self.write_camera_uniforms(
            self.camera_uniforms(),
            self.cloud_uniform_if_visible(!clouds_were_visible),
        );
    }

    fn replace_fog_environment(&mut self, environment: FogEnvironment) {
        self.fog_environment = environment.sanitized();
    }

    pub fn set_sky_environment(&mut self, environment: SkyEnvironment) {
        if !self.sky.set_environment(
            &self.device,
            &mut self.frame_uploads,
            environment,
            self.pack.celestial_atlas.as_ref(),
        ) {
            return;
        }
        self.update_camera();
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
        self.pack.end_sky_texture = Some(create_end_sky_texture_gpu(
            &self.device,
            &self.queue,
            &self.sky.end_sky_texture_layout,
            width,
            height,
            rgba,
            sampling,
        )?);
        Ok(())
    }

    pub fn upload_celestial_textures(&mut self, images: &[CelestialTextureImage]) -> Result<()> {
        self.pack.celestial_atlas = Some(create_celestial_atlas_gpu(
            &self.device,
            &self.queue,
            &self.sky.celestial_layout,
            images,
        )?);
        self.sky.refresh_celestials(
            &self.device,
            &mut self.frame_uploads,
            self.pack.celestial_atlas.as_ref(),
        );
        self.update_camera();
        Ok(())
    }

    pub fn upload_cloud_texture(&mut self, image: &CloudTextureImage) -> Result<()> {
        self.pack.cloud_texture = Some(create_cloud_texture_data(image)?);
        self.rebuild_clouds();
        Ok(())
    }

    pub fn upload_weather_textures(&mut self, images: &[WeatherTextureImage]) -> Result<()> {
        for image in images {
            let texture = create_weather_texture_gpu(
                &self.device,
                &self.queue,
                &self.terrain_bind_group_layout,
                &self.camera_buffers.world(),
                image,
            )?;
            match image.kind {
                WeatherTextureKind::Rain => self.pack.weather_rain_texture = Some(texture),
                WeatherTextureKind::Snow => self.pack.weather_snow_texture = Some(texture),
            }
        }
        Ok(())
    }

    pub fn set_weather_render_state(&mut self, state: WeatherRenderState) {
        self.weather_render_state = state;
    }

    /// Uploads the vanilla `textures/misc/forcefield.png` bytes
    /// (`WorldBorderRenderer.FORCEFIELD_LOCATION`);
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
        self.pack.world_border_texture = Some(create_world_border_texture_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
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

    pub fn set_data_driven_entity_rigs(&mut self, rigs: &[bbb_render_types::DataDrivenRigSpec]) {
        let baked: Vec<crate::entity_models::BakedDataDrivenRig> = rigs
            .iter()
            .map(crate::entity_models::BakedDataDrivenRig::from_spec)
            .collect();
        self.data_driven_rigs = baked.into();
        self.pack.entity_extract.clear();
        if !self.entity_model_instances.is_empty() && self.pack.entity_model_texture_atlas.is_some()
        {
            self.refresh_entity_model_meshes();
        }
    }

    pub fn set_dynamic_lights(
        &mut self,
        mode: bbb_render_types::DynamicLightsMode,
        mut lights: Vec<bbb_render_types::DynamicPointLight>,
    ) {
        if !mode.is_enabled() {
            lights.clear();
        } else if lights.len() > bbb_render_types::MAX_DYNAMIC_POINT_LIGHTS {
            lights.truncate(bbb_render_types::MAX_DYNAMIC_POINT_LIGHTS);
        }
        let mode_changed = self.dynamic_lights_mode != mode;
        let changed = mode_changed || self.dynamic_lights != lights;
        self.dynamic_lights_mode = mode;
        self.dynamic_lights = lights;
        if mode_changed {
            if let Ok(terrain) = crate::gpu::create_terrain_core_pipelines(
                &self.device,
                self.config.format,
                &self.terrain_bind_group_layout,
                &self.lightmap_sample_bind_group_layout,
                self.pipelines.shaders.terrain.as_ref(),
                self.terrain_texture_filtering,
                mode,
            ) {
                self.pipelines.terrain = terrain;
            }
        }
        if changed {
            self.update_camera();
        }
    }

    pub fn set_lod_terrain_mesh(
        &mut self,
        vertices: &[crate::scene::lod_terrain::LodTerrainVertex],
        indices: &[u32],
        lod_fog_end: f32,
    ) {
        self.lod_terrain
            .upload(&self.device, &self.queue, vertices, indices);
        let next_fog_end = if lod_fog_end.is_finite() {
            lod_fog_end.max(0.0)
        } else {
            0.0
        };
        if self.lod_fog_end.to_bits() != next_fog_end.to_bits() {
            self.lod_fog_end = next_fog_end;
            self.update_camera();
        }
    }

    pub fn clear_lod_terrain_mesh(&mut self) {
        self.lod_terrain.clear();
        if self.lod_fog_end != 0.0 {
            self.lod_fog_end = 0.0;
            self.update_camera();
        }
    }

    fn rebuild_clouds(&mut self) {
        self.rebuild_cloud_geometry();
        if self.has_cloud_draws() {
            self.write_cloud_uniform();
        }
    }

    pub(super) fn rebuild_cloud_geometry(&mut self) {
        self.pack.cloud_mesh_key = cloud_mesh_key(
            self.cloud_environment,
            self.pack.cloud_texture.as_ref(),
            self.cloud_frame,
            self.cloud_shape,
        );
        let next_clouds = create_cloud_gpu(
            &self.device,
            &mut self.cloud_quad_indices,
            self.cloud_environment,
            self.pack.cloud_texture.as_ref(),
            self.cloud_frame,
            self.cloud_shape,
        );
        if let Some(old) = std::mem::replace(&mut self.pack.clouds, next_clouds) {
            crate::frame_buffers::retire_gpu_buffer(&self.queue, old.face_buffer);
        }
    }

    pub(crate) fn has_cloud_draws(&self) -> bool {
        self.clouds_enabled && self.pack.clouds.is_some() && self.fog_environment.cloud_end > 0.0
    }

    fn current_cloud_uniform(&self) -> CloudUniform {
        cloud_uniform(
            self.cloud_environment,
            self.cloud_frame,
            self.pack.cloud_texture.as_ref(),
        )
    }

    fn cloud_uniform_if_visible(&self, changed: bool) -> Option<CloudUniform> {
        (changed && self.has_cloud_draws()).then(|| self.current_cloud_uniform())
    }

    pub(super) fn write_cloud_uniform(&self) {
        self.pending_cloud_uniform
            .set(Some(self.current_cloud_uniform()));
    }

    pub(crate) fn restore_cloud_uniform_after_resource_reload(&self) {
        self.write_cloud_uniform();
    }

    pub(super) fn scene_bounds(&self) -> Option<TerrainBounds> {
        let entity_bounds = self
            .pack
            .entity_meshes
            .bounds_meshes()
            .flat_map(|mesh| mesh.camera_bounds(&self.pack.entity_geometry));
        self.terrain
            .bounds()
            .into_iter()
            .chain(entity_bounds)
            .reduce(|mut bounds, other| {
                bounds.include_bounds(other);
                bounds
            })
    }
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
