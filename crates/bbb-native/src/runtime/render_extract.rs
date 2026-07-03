//! World -> renderer per-frame projections.
//!
//! Pure functions that read `WorldStore` (plus camera/effect context) and
//! produce renderer-facing state: lightmap and fog environments, clear/sky
//! colors, cloud frames, weather and lightning render state. No side
//! effects; everything GPU-facing stays in bbb-renderer.

use bbb_renderer::{
    BlockDestroyOverlay, EntityModelInstance, HudBlockItemModel, HudInventoryScreen,
    ItemEntityBillboard, ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture,
    ItemFrameMapSurface, ItemFrameMapTextSurface, ItemFrameMapTexture, ItemModelMesh, Renderer,
    SelectionOutline,
};

use super::*;

pub(crate) fn lightmap_environment_for_world(
    world: &WorldStore,
    brightness_factor: f32,
    block_factor: f32,
) -> LightmapEnvironment {
    lightmap_environment_for_world_at_tick(
        world,
        brightness_factor,
        block_factor,
        0,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    )
}

pub(crate) fn lightmap_environment_for_world_at_tick(
    world: &WorldStore,
    brightness_factor: f32,
    block_factor: f32,
    camera_tick_count: u64,
    partial_tick: f32,
) -> LightmapEnvironment {
    let darkness_effect_factor = local_player_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID)
        .map(vanilla_darkness_effect_factor)
        .unwrap_or(0.0);
    let night_vision_effect = local_player_effect(world, VANILLA_MOB_EFFECT_NIGHT_VISION_ID);
    let conduit_power_effect = local_player_effect(world, VANILLA_MOB_EFFECT_CONDUIT_POWER_ID);
    lightmap_environment_for_world_with_effects(
        world,
        brightness_factor,
        block_factor,
        camera_tick_count,
        partial_tick,
        darkness_effect_factor,
        night_vision_effect,
        conduit_power_effect,
        0.0,
        0.0,
        0.0,
        world.sky_flash_time() > 0,
    )
}

pub(crate) fn lightmap_environment_for_world_with_effects(
    world: &WorldStore,
    brightness_factor: f32,
    block_factor: f32,
    camera_tick_count: u64,
    partial_tick: f32,
    darkness_effect_factor: f32,
    night_vision_effect: Option<MobEffectState>,
    conduit_power_effect: Option<MobEffectState>,
    water_vision: f32,
    end_flash_sky_factor: f32,
    boss_overlay_world_darkening: f32,
    sky_flash_visible: bool,
) -> LightmapEnvironment {
    let mut environment = lightmap_environment_attributes_for_world(world);
    environment.sky_factor += end_flash_sky_factor;
    environment.boss_overlay_world_darkening = boss_overlay_world_darkening;
    let effects = local_player_lightmap_effects(
        brightness_factor,
        camera_tick_count,
        partial_tick,
        darkness_effect_factor,
        night_vision_effect,
        conduit_power_effect,
        water_vision,
    );
    environment.brightness_factor = effects.brightness_factor;
    environment.darkness_scale = effects.darkness_scale;
    environment.night_vision_factor = effects.night_vision_factor;
    environment.block_factor = block_factor;
    if sky_flash_visible {
        environment.sky_factor = 1.0;
    }
    environment.sanitized()
}

pub(crate) fn lightmap_environment_attributes_for_world(world: &WorldStore) -> LightmapEnvironment {
    let Some(level) = world.level_info() else {
        return LightmapEnvironment::default();
    };

    let mut environment = dimension_lightmap_environment(level);
    if vanilla_lightmap_dimension_kind(level) == VanillaLightmapDimensionKind::Overworld {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        apply_overworld_timeline_lightmap_environment(&mut environment, day_time);
        apply_weather_lightmap_environment(&mut environment, world.weather());
    }
    environment
}

pub(crate) fn particle_light_for_world(world: &WorldStore, position: [f64; 3]) -> [f32; 2] {
    let light = world
        .sample_block_light(particle_light_block_pos(position))
        .unwrap_or(VANILLA_PARTICLE_MISSING_CHUNK_LIGHT);
    [
        light.block.min(15) as f32 / 15.0,
        light.sky.min(15) as f32 / 15.0,
    ]
}

pub(crate) fn clear_color_for_world(world: &WorldStore, hide_lightning_flash: bool) -> ClearColor {
    clear_color_for_world_with_environment_colors(
        world,
        CameraEnvironmentColors::default(),
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        hide_lightning_flash,
    )
}

pub(crate) fn clear_color_for_world_at_camera(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_at_camera_with_render_distance(
        world,
        terrain_textures,
        camera_pose,
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        hide_lightning_flash,
    )
}

pub(crate) fn clear_color_for_world_at_camera_with_render_distance(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors(
        world,
        camera_environment_colors(world, terrain_textures, camera_pose),
        render_distance_chunks,
        hide_lightning_flash,
    )
}

pub(crate) fn clear_color_for_world_at_camera_with_water_vision(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    water_vision: f32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        camera_environment_colors(world, terrain_textures, camera_pose),
        render_distance_chunks,
        water_vision,
        hide_lightning_flash,
    )
}

pub(crate) fn clear_color_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        colors,
        render_distance_chunks,
        0.0,
        hide_lightning_flash,
    )
}

pub(crate) fn clear_color_for_world_with_environment_colors_and_water_vision(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    water_vision: f32,
    hide_lightning_flash: bool,
) -> ClearColor {
    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let weather = world.weather();
    let rain = weather.rain_level.clamp(0.0, 1.0) as f64;
    let thunder = weather.thunder_level.clamp(0.0, 1.0) as f64;
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);
    let clear = match colors.fog_type {
        CameraFogType::Atmospheric => clear_color_for_day_time_with_environment_colors_and_camera(
            day_time,
            rain,
            thunder,
            colors
                .fog_color
                .or_else(|| dimension_fog_color_for_kind(dimension_kind)),
            colors
                .sky_color
                .or_else(|| dimension_sky_color_for_kind(dimension_kind)),
            dimension_kind,
            colors.camera_forward,
            render_distance_chunks,
        ),
        CameraFogType::Water => clear_color_from_argb(apply_fog_brightening(
            rgb_u8_to_argb(
                colors
                    .water_fog_color
                    .unwrap_or(VANILLA_DEFAULT_WATER_FOG_COLOR),
            ),
            water_vision,
        )),
    };
    if hide_lightning_flash
        || world.sky_flash_time() == 0
        || colors.fog_type != CameraFogType::Atmospheric
    {
        clear
    } else {
        clear_color_with_sky_flash(clear)
    }
}

pub(crate) fn sky_environment_for_world_at_camera(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    hide_lightning_flash: bool,
) -> SkyEnvironment {
    sky_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors(world, terrain_textures, camera_pose),
        hide_lightning_flash,
    )
}

pub(crate) fn sky_environment_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    hide_lightning_flash: bool,
) -> SkyEnvironment {
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);
    if dimension_kind == VanillaLightmapDimensionKind::End {
        return SkyEnvironment::end();
    }
    if dimension_kind != VanillaLightmapDimensionKind::Overworld {
        return SkyEnvironment::disabled();
    }

    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let weather = world.weather();
    let rain = weather.rain_level.clamp(0.0, 1.0) as f64;
    let thunder = weather.thunder_level.clamp(0.0, 1.0) as f64;
    let sunrise_sunset_color = apply_weather_sunrise_sunset_color_layers(
        sample_periodic_argb_keyframes(
            day_time,
            &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        ),
        rain,
        thunder,
    );

    SkyEnvironment::from_rgb(rgb24(sky_disc_color_for_world_with_environment_colors(
        world,
        colors,
        dimension_kind,
        hide_lightning_flash,
    )))
    .with_sunrise_sunset(
        rgba32(sunrise_sunset_color),
        overworld_sun_angle(day_time).to_radians(),
    )
    .with_celestial_state(
        overworld_moon_angle(day_time).to_radians(),
        1.0 - weather.rain_level.clamp(0.0, 1.0),
        overworld_moon_phase(day_time),
    )
    .with_star_state(
        overworld_star_angle(day_time).to_radians(),
        apply_weather_star_brightness_layers(
            sample_periodic_float_keyframes(
                day_time,
                &VANILLA_OVERWORLD_STAR_BRIGHTNESS_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
            weather.rain_level.clamp(0.0, 1.0),
            weather.thunder_level.clamp(0.0, 1.0),
        ),
    )
}

pub(crate) fn cloud_environment_for_world(world: &WorldStore) -> CloudEnvironment {
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);
    if dimension_kind == VanillaLightmapDimensionKind::Overworld {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        let weather = world.weather();
        let cloud_color = apply_weather_cloud_color_layers(
            argb_multiply(
                rgba01_to_argb(VANILLA_DEFAULT_CLOUD_COLOR),
                sample_periodic_argb_keyframes(
                    day_time,
                    &VANILLA_OVERWORLD_CLOUD_COLOR_MULTIPLIER_KEYFRAMES,
                    VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                ),
            ),
            weather.rain_level as f64,
            weather.thunder_level as f64,
        );
        CloudEnvironment::with_color_and_height(rgba32(cloud_color), VANILLA_DEFAULT_CLOUD_HEIGHT)
    } else {
        CloudEnvironment::disabled()
    }
}

pub(crate) fn cloud_frame_for_world(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    partial_tick: f32,
) -> CloudFrame {
    let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
    camera_pose
        .map(|pose| CloudFrame::from_camera_pose(pose, game_time, partial_tick))
        .unwrap_or_else(|| CloudFrame::at_camera_position([0.0, 0.0, 0.0], game_time, partial_tick))
}

pub(crate) fn weather_render_state_for_world(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    partial_tick: f32,
) -> WeatherRenderState {
    let Some(camera_pose) = camera_pose else {
        return WeatherRenderState::default();
    };
    let camera_position = camera_eye_position(camera_pose);
    if !camera_position.into_iter().all(f32::is_finite) {
        return WeatherRenderState::default();
    }
    let lightning_bolts = lightning_bolts_for_world(world);
    if !world_can_have_weather(world) {
        return WeatherRenderState::with_lightning(
            WeatherFrame::default(),
            Vec::new(),
            Vec::new(),
            lightning_bolts,
        );
    }
    let rain_level = world.weather().rain_level.clamp(0.0, 1.0);
    let frame = WeatherFrame::at_camera_position(
        camera_position,
        VANILLA_WEATHER_RENDER_RADIUS,
        rain_level,
    );
    if rain_level <= 0.0 {
        return WeatherRenderState::with_lightning(frame, Vec::new(), Vec::new(), lightning_bolts);
    }
    let camera_block_x = camera_position[0].floor() as i32;
    let camera_block_y = camera_position[1].floor() as i32;
    let camera_block_z = camera_position[2].floor() as i32;
    let radius = VANILLA_WEATHER_RENDER_RADIUS as i32;
    let sea_level = world
        .level_info()
        .map(|level| level.sea_level)
        .unwrap_or(63);
    let ticks = world.world_time().map(|time| time.game_time).unwrap_or(0) as i32;
    let mut rain_columns = Vec::new();
    let mut snow_columns = Vec::new();

    for z in (camera_block_z - radius)..=(camera_block_z + radius) {
        for x in (camera_block_x - radius)..=(camera_block_x + radius) {
            let sample_pos = BlockPos {
                x,
                y: camera_block_y,
                z,
            };
            let Some(precipitation) =
                weather_precipitation_at(world, terrain_textures, sample_pos, sea_level)
            else {
                continue;
            };
            let terrain_height =
                weather_motion_blocking_height(world, x, z).unwrap_or(camera_block_y - radius);
            let bottom_y = (camera_block_y - radius).max(terrain_height);
            let top_y = (camera_block_y + radius).max(terrain_height);
            if top_y - bottom_y == 0 {
                continue;
            }

            let light_sample_y = camera_block_y.max(terrain_height);
            let raw_light = world
                .sample_block_light(BlockPos {
                    x,
                    y: light_sample_y,
                    z,
                })
                .unwrap_or(VANILLA_PARTICLE_MISSING_CHUNK_LIGHT);
            match precipitation {
                WeatherPrecipitation::Rain => rain_columns.push(rain_weather_column(
                    x,
                    z,
                    bottom_y,
                    top_y,
                    raw_light,
                    ticks,
                    partial_tick,
                )),
                WeatherPrecipitation::Snow => snow_columns.push(snow_weather_column(
                    x,
                    z,
                    bottom_y,
                    top_y,
                    raw_light,
                    ticks,
                    partial_tick,
                )),
            }
        }
    }

    WeatherRenderState::with_lightning(frame, rain_columns, snow_columns, lightning_bolts)
}

pub(crate) fn lightning_bolts_for_world(world: &WorldStore) -> Vec<LightningBoltRenderState> {
    world
        .entity_transforms()
        .into_iter()
        .filter(|entity| entity.entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID)
        .filter_map(|entity| {
            let position = [
                entity.position.x as f32,
                entity.position.y as f32,
                entity.position.z as f32,
            ];
            position
                .into_iter()
                .all(f32::is_finite)
                .then(|| LightningBoltRenderState {
                    position,
                    seed: lightning_bolt_seed(entity.uuid),
                })
        })
        .collect()
}

pub(crate) fn sky_disc_color_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    dimension_kind: VanillaLightmapDimensionKind,
    hide_lightning_flash: bool,
) -> i32 {
    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let weather = world.weather();
    let rain = weather.rain_level.clamp(0.0, 1.0) as f64;
    let thunder = weather.thunder_level.clamp(0.0, 1.0) as f64;
    let mut sky_color = rgb_u8_to_argb(
        colors
            .sky_color
            .or_else(|| dimension_sky_color_for_kind(dimension_kind))
            .unwrap_or([0, 0, 0]),
    );

    if dimension_kind == VanillaLightmapDimensionKind::Overworld {
        sky_color = argb_multiply(
            sky_color,
            sample_periodic_argb_keyframes(
                day_time,
                &VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
        );
    }
    sky_color = apply_atmospheric_sky_weather_darken(sky_color, rain, thunder);
    if !hide_lightning_flash && world.sky_flash_time() > 0 {
        sky_color = argb_srgb_lerp(
            VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
            sky_color,
            VANILLA_SKY_FLASH_SKY_COLOR,
        );
    }
    sky_color
}

pub(crate) fn fog_environment_for_world_at_camera(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    water_vision: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
) -> FogEnvironment {
    fog_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors(world, terrain_textures, camera_pose),
        render_distance_chunks,
        water_vision,
        rain_fog_multiplier,
        hide_lightning_flash,
    )
}

pub(crate) fn fog_environment_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    water_vision: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
) -> FogEnvironment {
    let fog_color = clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        colors,
        render_distance_chunks,
        water_vision,
        hide_lightning_flash,
    );
    let color = [
        fog_color.r as f32,
        fog_color.g as f32,
        fog_color.b as f32,
        fog_color.a as f32,
    ];
    match colors.fog_type {
        CameraFogType::Atmospheric => {
            let dimension_kind = world
                .level_info()
                .map(vanilla_lightmap_dimension_kind)
                .unwrap_or(VanillaLightmapDimensionKind::Overworld);
            let (mut environmental_start, mut environmental_end) =
                atmospheric_fog_distance_for_dimension(dimension_kind);
            let mut sky_end = atmospheric_sky_fog_end(render_distance_chunks);
            let mut cloud_end = atmospheric_cloud_fog_end();
            apply_atmospheric_rain_fog_distance(
                &mut environmental_start,
                &mut environmental_end,
                rain_fog_multiplier,
            );
            if world.boss_overlay_should_create_world_fog() {
                environmental_start = environmental_start.min(VANILLA_NETHER_FOG_START_DISTANCE);
                environmental_end = environmental_end.min(VANILLA_NETHER_FOG_END_DISTANCE);
                sky_end = environmental_end;
                cloud_end = environmental_end;
            }
            FogEnvironment::world_with_visibility_ends(
                color,
                environmental_start,
                environmental_end,
                render_distance_chunks,
                sky_end,
                cloud_end,
            )
        }
        CameraFogType::Water => {
            let environmental_end = colors
                .water_fog_end_distance
                .unwrap_or(VANILLA_DEFAULT_WATER_FOG_END_DISTANCE)
                * water_vision.max(0.25);
            FogEnvironment::world_with_visibility_ends(
                color,
                VANILLA_DEFAULT_WATER_FOG_START_DISTANCE,
                environmental_end,
                render_distance_chunks,
                environmental_end,
                environmental_end,
            )
        }
    }
}

/// One frame's worth of world->renderer state.
///
/// `pump_network_and_terrain` extracts each field at a vanilla-verified
/// client-tick sequence point, then commits the whole frame to the renderer in
/// one `apply_renderer_frame` call. Sky-flash-dependent environment fields read
/// after the `ClientLevel.tick`-equivalent `advance_sky_flash_time`.
pub(crate) struct RendererFrame {
    pub(crate) lightmap_environment: LightmapEnvironment,
    pub(crate) clear_color: ClearColor,
    pub(crate) fog_environment: FogEnvironment,
    pub(crate) sky_environment: SkyEnvironment,
    pub(crate) cloud_environment: CloudEnvironment,
    pub(crate) hud_health: Option<f32>,
    pub(crate) hud_food: Option<i32>,
    pub(crate) hud_experience_progress: Option<f32>,
    pub(crate) hud_selected_slot: u8,
    pub(crate) hud_hotbar_item_icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS],
    pub(crate) hud_hotbar_block_item_models: Vec<Option<HudBlockItemModel>>,
    pub(crate) hud_inventory_screen: Option<HudInventoryScreen>,
    pub(crate) item_entity_billboards: Vec<ItemEntityBillboard>,
    pub(crate) block_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) block_item_model_z_offset_forward_meshes: Vec<ItemModelMesh>,
    pub(crate) block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(crate) item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_block_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_block_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_flat_item_model_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_flat_item_model_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_item_model_glint_meshes: Vec<ItemModelMesh>,
    pub(crate) first_person_item_model_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub(crate) item_frame_map_textures: Vec<ItemFrameMapTexture>,
    pub(crate) item_frame_map_surfaces: Vec<ItemFrameMapSurface>,
    pub(crate) item_frame_map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub(crate) item_frame_map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub(crate) item_frame_map_text_surfaces: Vec<ItemFrameMapTextSurface>,
    pub(crate) entity_model_instances: Vec<EntityModelInstance>,
    pub(crate) camera_pose: Option<CameraPose>,
    pub(crate) cloud_frame: CloudFrame,
    pub(crate) weather_render_state: WeatherRenderState,
    pub(crate) selection_outline: Option<SelectionOutline>,
    pub(crate) entity_scene_outline: Option<SelectionOutline>,
    pub(crate) entity_target_outline: Option<SelectionOutline>,
    pub(crate) block_destroy_overlays: Vec<BlockDestroyOverlay>,
}

/// Commits one extracted frame to the renderer in a single call.
pub(crate) fn apply_renderer_frame(renderer: &mut Renderer, frame: RendererFrame) {
    renderer.set_lightmap_environment(frame.lightmap_environment);
    renderer.set_clear_color(frame.clear_color);
    renderer.set_fog_environment(frame.fog_environment);
    renderer.set_sky_environment(frame.sky_environment);
    renderer.set_cloud_environment(frame.cloud_environment);
    renderer.set_hud_health(frame.hud_health);
    renderer.set_hud_food(frame.hud_food);
    renderer.set_hud_experience_progress(frame.hud_experience_progress);
    renderer.set_hud_selected_slot(frame.hud_selected_slot);
    renderer.set_hud_hotbar_item_icons(frame.hud_hotbar_item_icons);
    renderer.set_hud_hotbar_block_item_models(frame.hud_hotbar_block_item_models);
    renderer.set_hud_inventory_screen(frame.hud_inventory_screen);
    renderer.set_item_entity_billboards(frame.item_entity_billboards);
    renderer.set_block_item_model_meshes(frame.block_item_model_meshes);
    renderer.set_block_item_model_z_offset_forward_meshes(
        frame.block_item_model_z_offset_forward_meshes,
    );
    renderer.set_block_item_model_translucent_meshes(frame.block_item_model_translucent_meshes);
    renderer.set_flat_item_model_meshes(frame.flat_item_model_meshes);
    renderer.set_flat_item_model_translucent_meshes(frame.flat_item_model_translucent_meshes);
    renderer.set_item_model_glint_meshes(frame.item_model_glint_meshes);
    renderer.set_item_model_glint_translucent_meshes(frame.item_model_glint_translucent_meshes);
    renderer.set_first_person_block_item_model_meshes(frame.first_person_block_item_model_meshes);
    renderer.set_first_person_block_item_model_translucent_meshes(
        frame.first_person_block_item_model_translucent_meshes,
    );
    renderer.set_first_person_flat_item_model_meshes(frame.first_person_flat_item_model_meshes);
    renderer.set_first_person_flat_item_model_translucent_meshes(
        frame.first_person_flat_item_model_translucent_meshes,
    );
    renderer.set_first_person_item_model_glint_meshes(frame.first_person_item_model_glint_meshes);
    renderer.set_first_person_item_model_glint_translucent_meshes(
        frame.first_person_item_model_glint_translucent_meshes,
    );
    renderer
        .set_item_frame_map_surfaces(frame.item_frame_map_textures, frame.item_frame_map_surfaces);
    renderer.set_item_frame_map_decoration_surfaces(
        frame.item_frame_map_decoration_textures,
        frame.item_frame_map_decoration_surfaces,
    );
    renderer.set_item_frame_map_text_surfaces(frame.item_frame_map_text_surfaces);
    renderer.set_entity_model_instances(frame.entity_model_instances);
    renderer.set_camera_pose(frame.camera_pose);
    renderer.set_cloud_frame(frame.cloud_frame);
    renderer.set_weather_render_state(frame.weather_render_state);
    renderer.set_selection_outline(frame.selection_outline);
    renderer.set_entity_scene_outline(frame.entity_scene_outline);
    renderer.set_entity_target_outline(frame.entity_target_outline);
    renderer.set_block_destroy_overlays(frame.block_destroy_overlays);
}
