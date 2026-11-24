use super::*;

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn lightmap_environment_for_world_at_tick(
    world: &WorldStore,
    brightness_factor: f32,
    block_factor: f32,
    camera_tick_count: u64,
    partial_tick: f32,
) -> LightmapEnvironment {
    let darkness_effect_factor = local_player_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID)
        .map(|effect| effect.blend_factor(partial_tick))
        .unwrap_or(0.0);
    let night_vision_effect = local_player_effect(world, VANILLA_MOB_EFFECT_NIGHT_VISION_ID);
    let conduit_power_effect = local_player_effect(world, VANILLA_MOB_EFFECT_CONDUIT_POWER_ID);
    lightmap_environment_for_world_with_effects(
        world,
        LightmapEffectInputs {
            brightness_factor,
            block_factor,
            camera_tick_count,
            partial_tick,
            darkness_effect_factor,
            night_vision_effect,
            conduit_power_effect,
            water_vision: 0.0,
            end_flash_sky_factor: 0.0,
            boss_overlay_world_darkening: 0.0,
            sky_flash_visible: world.sky_flash_time() > 0,
        },
    )
}

pub(crate) struct LightmapEffectInputs {
    pub(crate) brightness_factor: f32,
    pub(crate) block_factor: f32,
    pub(crate) camera_tick_count: u64,
    pub(crate) partial_tick: f32,
    pub(crate) darkness_effect_factor: f32,
    pub(crate) night_vision_effect: Option<MobEffectState>,
    pub(crate) conduit_power_effect: Option<MobEffectState>,
    pub(crate) water_vision: f32,
    pub(crate) end_flash_sky_factor: f32,
    pub(crate) boss_overlay_world_darkening: f32,
    pub(crate) sky_flash_visible: bool,
}

pub(crate) fn lightmap_environment_for_world_with_effects(
    world: &WorldStore,
    inputs: LightmapEffectInputs,
) -> LightmapEnvironment {
    let LightmapEffectInputs {
        brightness_factor,
        block_factor,
        camera_tick_count,
        partial_tick,
        darkness_effect_factor,
        night_vision_effect,
        conduit_power_effect,
        water_vision,
        end_flash_sky_factor,
        boss_overlay_world_darkening,
        sky_flash_visible,
    } = inputs;
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

    let sampled_attributes =
        sampled_full_environment_attributes_at_camera(world, camera_pose_from_world(world));
    let mut environment = dimension_lightmap_environment(level, sampled_attributes);
    let is_overworld =
        vanilla_lightmap_dimension_kind(level) == VanillaLightmapDimensionKind::Overworld;
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    if is_overworld && uses_builtin_dimension_fallback {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        apply_overworld_timeline_lightmap_environment(&mut environment, day_time);
    }
    if level.can_have_weather() {
        apply_weather_lightmap_environment(&mut environment, world.weather());
    }
    environment
}

pub(crate) fn particle_light_for_world(world: &WorldStore, position: [f64; 3]) -> [f32; 2] {
    world
        .sample_render_light(particle_light_block_pos(position))
        .unwrap_or(VanillaLightCoords::MISSING_CHUNK_PARTICLE)
        .normalized()
}

#[cfg(test)]
pub(crate) fn clear_color_for_world(world: &WorldStore, hide_lightning_flash: bool) -> ClearColor {
    clear_color_for_world_with_environment_colors(
        world,
        CameraEnvironmentColors::default(),
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        hide_lightning_flash,
    )
}

#[cfg(test)]
pub(crate) fn clear_color_for_world_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_at_camera_with_render_distance(
        world,
        camera_pose,
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
        hide_lightning_flash,
    )
}

#[cfg(test)]
pub(crate) fn clear_color_for_world_at_camera_with_render_distance(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors(
        world,
        camera_environment_colors_for_default_projection(world, camera_pose),
        render_distance_chunks,
        hide_lightning_flash,
    )
}

#[cfg(test)]
pub(crate) fn clear_color_for_world_at_camera_with_water_vision(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    water_vision: f32,
    hide_lightning_flash: bool,
) -> ClearColor {
    clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        camera_environment_colors_for_default_projection(world, camera_pose),
        render_distance_chunks,
        water_vision,
        hide_lightning_flash,
    )
}

#[cfg(test)]
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

/// Vanilla `Level.getRainLevel` / `Level.getThunderLevel` (`Level.java:913-915`):
/// thunder scales by rain level and dimensions without weather stay clear.
fn effective_weather_levels(world: &WorldStore) -> (f64, f64) {
    let can_have_weather = world
        .level_info()
        .is_none_or(WorldLevelInfo::can_have_weather);
    if !can_have_weather {
        return (0.0, 0.0);
    }
    let weather = world.weather();
    let rain = weather.rain_level.clamp(0.0, 1.0) as f64;
    let thunder = (weather.thunder_level.clamp(0.0, 1.0) as f64) * rain;
    (rain, thunder)
}

pub(crate) fn clear_color_for_world_with_environment_colors_and_water_vision(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    water_vision: f32,
    hide_lightning_flash: bool,
) -> ClearColor {
    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let (rain, thunder) = effective_weather_levels(world);
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    let clear = match colors.fog_type {
        CameraFogType::Atmospheric => {
            clear_color_for_atmospheric_environment(AtmosphericClearColorInputs {
                day_time,
                rain_level: rain,
                thunder_level: thunder,
                fog_color: colors
                    .fog_color
                    .or_else(|| dimension_fog_color_for_kind(dimension_kind)),
                sky_color: colors
                    .sky_color
                    .or_else(|| dimension_sky_color_for_kind(dimension_kind)),
                sun_angle_degrees: colors.sun_angle_degrees,
                sunrise_sunset_color: colors.sunrise_sunset_color,
                sky_fog_end_distance: colors.sky_fog_end_distance,
                dimension_kind,
                uses_builtin_dimension_fallback,
                camera_forward: colors.camera_forward,
                render_distance_chunks,
            })
        }
        CameraFogType::Water => clear_color_from_argb(rgb_u8_to_argb(
            colors
                .water_fog_color
                .unwrap_or(VANILLA_DEFAULT_WATER_FOG_COLOR),
        )),
        CameraFogType::Lava => clear_color_from_argb(VANILLA_LAVA_FOG_COLOR),
        CameraFogType::PowderSnow => clear_color_from_argb(VANILLA_POWDER_SNOW_FOG_COLOR),
    };
    let clear = if hide_lightning_flash
        || world.sky_flash_time() == 0
        || colors.fog_type != CameraFogType::Atmospheric
    {
        clear
    } else {
        clear_color_with_sky_flash(clear)
    };
    apply_camera_mob_effect_fog_color(
        world,
        clear,
        colors.fog_type,
        water_vision,
        colors.camera_eye_y,
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
    )
}

pub(super) fn apply_camera_mob_effect_fog_color(
    world: &WorldStore,
    clear: ClearColor,
    fog_type: CameraFogType,
    water_vision: f32,
    camera_eye_y: Option<f32>,
    partial_tick: f32,
) -> ClearColor {
    // Vanilla `FogRenderer.computeFogColor` seeds the darkness factor with the
    // void-darkness term clamp((onsetRange + minY - camera.y) / onsetRange)
    // before the blindness/darkness fog environments raise it via max().
    let void_darkness = camera_eye_y.map_or(0.0, |camera_y| {
        let onset_range = world.client_void_darkness_onset_range();
        ((onset_range + world.dimension().min_y as f32 - camera_y) / onset_range).clamp(0.0, 1.0)
    });
    let blindness = camera_entity_effect(world, VANILLA_MOB_EFFECT_BLINDNESS_ID);
    let darkness_effect = camera_entity_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID);
    let darkness = blindness.map_or_else(
        || {
            darkness_effect
                .map(|effect| effect.blend_factor(partial_tick).max(void_darkness))
                .unwrap_or(void_darkness)
        },
        |effect| {
            if mob_effect_ends_within(effect, 19) {
                (effect.duration_ticks as f32 / 20.0).max(void_darkness)
            } else {
                1.0
            }
        },
    );
    let mut color = clear_color_to_argb(clear);
    if darkness > 0.0 && !matches!(fog_type, CameraFogType::Lava | CameraFogType::PowderSnow) {
        let brightness = (1.0 - darkness.clamp(0.0, 1.0)).powi(2);
        color = argb_scale_rgb(color, brightness, brightness, brightness);
    }

    let brighten_factor = match fog_type {
        CameraFogType::Water => {
            if world.local_player().camera.follows_player {
                water_vision
            } else {
                1.0
            }
        }
        CameraFogType::Atmospheric | CameraFogType::Lava | CameraFogType::PowderSnow
            if darkness_effect.is_none() =>
        {
            camera_entity_effect(world, VANILLA_MOB_EFFECT_NIGHT_VISION_ID)
                .map(|effect| vanilla_night_vision_scale(effect, partial_tick))
                .unwrap_or(0.0)
        }
        CameraFogType::Atmospheric | CameraFogType::Lava | CameraFogType::PowderSnow => 0.0,
    };
    clear_color_from_argb(apply_fog_brightening(color, brighten_factor))
}

#[cfg(test)]
pub(crate) fn sky_environment_for_world_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    hide_lightning_flash: bool,
) -> SkyEnvironment {
    sky_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors_for_default_projection(world, camera_pose),
        hide_lightning_flash,
    )
}

pub(crate) fn sky_environment_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    hide_lightning_flash: bool,
) -> SkyEnvironment {
    // `GameRenderer.renderLevel` passes `!shouldCreateBossFog` as
    // `LevelRenderer.renderLevel`'s `shouldRenderSky` argument. Boss fog only
    // suppresses the sky pass; atmospheric fog and End-flash light remain
    // separate consumers of the same aggregate property.
    // `LevelRenderer.addSkyPass` keeps underwater sky visible but rejects
    // Lava and powder snow before scheduling any skybox pass.
    if matches!(
        colors.fog_type,
        CameraFogType::Lava | CameraFogType::PowderSnow
    ) || world.boss_overlay_should_create_world_fog()
        || camera_mob_effect_blocks_sky(world)
    {
        return SkyEnvironment::disabled();
    }
    match world
        .level_info()
        .map(WorldLevelInfo::skybox)
        .unwrap_or(WorldSkybox::Overworld)
    {
        WorldSkybox::None => return SkyEnvironment::disabled(),
        WorldSkybox::End => return SkyEnvironment::end(),
        WorldSkybox::Overworld => {}
    }
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);

    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    let (rain, thunder) = effective_weather_levels(world);
    let sunrise_sunset_color = apply_weather_sunrise_sunset_color_layers(
        colors
            .sunrise_sunset_color
            .map(rgba_u8_to_argb)
            .unwrap_or_else(|| {
                if uses_builtin_dimension_fallback {
                    sample_periodic_argb_keyframes(
                        day_time,
                        &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
                        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                    )
                } else {
                    0
                }
            }),
        rain,
        thunder,
    );
    let moon_phase = colors.moon_phase.map(client_moon_phase).unwrap_or_else(|| {
        if uses_builtin_dimension_fallback {
            overworld_moon_phase(day_time)
        } else {
            SkyMoonPhase::FullMoon
        }
    });

    SkyEnvironment::from_rgb(rgb24(sky_disc_color_for_world_with_environment_colors(
        world,
        colors,
        dimension_kind,
        hide_lightning_flash,
    )))
    .with_sunrise_sunset(
        rgba32(sunrise_sunset_color),
        colors
            .sun_angle_degrees
            .unwrap_or_else(|| {
                if uses_builtin_dimension_fallback {
                    overworld_sun_angle(day_time)
                } else {
                    0.0
                }
            })
            .to_radians(),
    )
    .with_celestial_state(
        colors
            .moon_angle_degrees
            .unwrap_or_else(|| {
                if uses_builtin_dimension_fallback {
                    overworld_moon_angle(day_time)
                } else {
                    0.0
                }
            })
            .to_radians(),
        1.0 - rain as f32,
        moon_phase,
    )
    .with_star_state(
        colors
            .star_angle_degrees
            .unwrap_or_else(|| {
                if uses_builtin_dimension_fallback {
                    overworld_star_angle(day_time)
                } else {
                    0.0
                }
            })
            .to_radians(),
        apply_weather_star_brightness_layers(
            colors.star_brightness.unwrap_or_else(|| {
                if uses_builtin_dimension_fallback {
                    sample_periodic_float_keyframes(
                        day_time,
                        &VANILLA_OVERWORLD_STAR_BRIGHTNESS_KEYFRAMES,
                        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                    )
                } else {
                    0.0
                }
            }),
            rain as f32,
            thunder as f32,
        ),
    )
    .with_dark_disc(world.local_player_eye_below_horizon())
}

pub(crate) fn cloud_environment_for_world(world: &WorldStore) -> CloudEnvironment {
    let dimension_kind = world
        .level_info()
        .map(vanilla_lightmap_dimension_kind)
        .unwrap_or(VanillaLightmapDimensionKind::Overworld);
    let attributes =
        sampled_full_environment_attributes_at_camera(world, camera_pose_from_world(world));
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    if attributes.cloud_color.is_some()
        || uses_builtin_dimension_fallback
            && dimension_kind == VanillaLightmapDimensionKind::Overworld
    {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        let (rain, thunder) = effective_weather_levels(world);
        let cloud_color = apply_weather_cloud_color_layers(
            attributes
                .cloud_color
                .map(rgba_u8_to_argb)
                .unwrap_or_else(|| {
                    argb_multiply(
                        rgba01_to_argb(VANILLA_DEFAULT_CLOUD_COLOR),
                        sample_periodic_argb_keyframes(
                            day_time,
                            &VANILLA_OVERWORLD_CLOUD_COLOR_MULTIPLIER_KEYFRAMES,
                            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                        ),
                    )
                }),
            rain,
            thunder,
        );
        CloudEnvironment::with_color_and_height(
            rgba32(cloud_color),
            attributes
                .cloud_height
                .unwrap_or(VANILLA_DEFAULT_CLOUD_HEIGHT),
        )
    } else {
        CloudEnvironment::disabled()
    }
}

pub(super) fn rgba_u8_to_argb([red, green, blue, alpha]: [u8; 4]) -> i32 {
    i32::from_be_bytes([alpha, red, green, blue])
}

pub(super) fn client_moon_phase(phase: bbb_client_world::ClientMoonPhase) -> SkyMoonPhase {
    match phase {
        bbb_client_world::ClientMoonPhase::FullMoon => SkyMoonPhase::FullMoon,
        bbb_client_world::ClientMoonPhase::WaningGibbous => SkyMoonPhase::WaningGibbous,
        bbb_client_world::ClientMoonPhase::ThirdQuarter => SkyMoonPhase::ThirdQuarter,
        bbb_client_world::ClientMoonPhase::WaningCrescent => SkyMoonPhase::WaningCrescent,
        bbb_client_world::ClientMoonPhase::NewMoon => SkyMoonPhase::NewMoon,
        bbb_client_world::ClientMoonPhase::WaxingCrescent => SkyMoonPhase::WaxingCrescent,
        bbb_client_world::ClientMoonPhase::FirstQuarter => SkyMoonPhase::FirstQuarter,
        bbb_client_world::ClientMoonPhase::WaxingGibbous => SkyMoonPhase::WaxingGibbous,
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

#[cfg(test)]
pub(crate) fn weather_render_state_for_world(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    partial_tick: f32,
) -> WeatherRenderState {
    weather_render_state_for_world_with_radius(
        world,
        terrain_textures,
        camera_pose,
        partial_tick,
        VANILLA_WEATHER_RENDER_RADIUS,
    )
}

pub(crate) fn weather_render_state_for_world_with_radius(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    partial_tick: f32,
    weather_radius: u32,
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
    let frame =
        WeatherFrame::at_camera_position(camera_position, weather_radius.clamp(3, 10), rain_level);
    if rain_level <= 0.0 {
        return WeatherRenderState::with_lightning(frame, Vec::new(), Vec::new(), lightning_bolts);
    }
    let camera_block_x = camera_position[0].floor() as i32;
    let camera_block_y = camera_position[1].floor() as i32;
    let camera_block_z = camera_position[2].floor() as i32;
    let radius = weather_radius.clamp(3, 10) as i32;
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
            let light = world
                .sample_render_light(BlockPos {
                    x,
                    y: light_sample_y,
                    z,
                })
                .unwrap_or(VanillaLightCoords::FULL_BRIGHT);
            match precipitation {
                WeatherPrecipitation::Rain => rain_columns.push(rain_weather_column(
                    x,
                    z,
                    bottom_y,
                    top_y,
                    light,
                    ticks,
                    partial_tick,
                )),
                WeatherPrecipitation::Snow => snow_columns.push(snow_weather_column(
                    x,
                    z,
                    bottom_y,
                    top_y,
                    light,
                    ticks,
                    partial_tick,
                )),
            }
        }
    }

    WeatherRenderState::with_lightning(frame, rain_columns, snow_columns, lightning_bolts)
}

/// Projects the client world border into the renderer's world border state,
/// transcribing vanilla `WorldBorderRenderer.extract`
/// plus the render-call inputs vanilla
/// threads alongside the state (`LevelRenderer.addWeatherPass`,
/// `Camera`, `WorldBorderRenderer.render`).
///
/// `scroll_millis` is the wall-clock milliseconds feeding vanilla's
/// `Util.getMillis() % 3000L` forcefield UV scroll.
pub(crate) fn world_border_render_state_for_world(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    depth_far: f32,
    partial_tick: f32,
    scroll_millis: u64,
) -> WorldBorderRenderState {
    let Some(camera_pose) = camera_pose else {
        return WorldBorderRenderState::default();
    };
    let eye = camera_eye_position(camera_pose);
    if !eye.into_iter().all(f32::is_finite) {
        return WorldBorderRenderState::default();
    }
    let camera = [f64::from(eye[0]), f64::from(eye[1]), f64::from(eye[2])];
    let border = world.world_border();
    // Vanilla passes `options.getEffectiveRenderDistance() * 16` blocks to both
    // extract and render (LevelRenderer.addWeatherPass:744).
    let render_distance = f64::from(render_distance_chunks) * 16.0;
    // Vanilla forcefield UV scroll offset.
    let texture_offset = (scroll_millis % 3000) as f32 / 3000.0;

    // WorldBorderRenderer.extract: the state
    // bounds use the frame partial tick.
    let min_x = border.min_x_at(partial_tick);
    let max_x = border.max_x_at(partial_tick);
    let min_z = border.min_z_at(partial_tick);
    let max_z = border.max_z_at(partial_tick);
    // Visibility condition: the camera is
    // within renderDistance of some border edge (not strictly inside the
    // shrunk box) AND not further than renderDistance outside the border.
    use std::cmp::Ordering;
    let near_some_edge = camera[0].partial_cmp(&(max_x - render_distance)) != Some(Ordering::Less)
        || camera[0].partial_cmp(&(min_x + render_distance)) != Some(Ordering::Greater)
        || camera[2].partial_cmp(&(max_z - render_distance)) != Some(Ordering::Less)
        || camera[2].partial_cmp(&(min_z + render_distance)) != Some(Ordering::Greater);
    let inside_expanded_box = camera[0].partial_cmp(&(min_x - render_distance))
        != Some(Ordering::Less)
        && camera[0].partial_cmp(&(max_x + render_distance)) != Some(Ordering::Greater)
        && camera[2].partial_cmp(&(min_z - render_distance)) != Some(Ordering::Less)
        && camera[2].partial_cmp(&(max_z + render_distance)) != Some(Ordering::Greater);
    let (alpha, tint) = if near_some_edge && inside_expanded_box {
        // alpha = clamp((1 - distanceToBorder / renderDistance)^4, 0, 1);
        // tint = border.getStatus().getColor()
        let alpha = (1.0 - border.distance_to_border(camera[0], camera[2]) / render_distance)
            .powi(4)
            .clamp(0.0, 1.0);
        (alpha, border.status().color())
    } else {
        (0.0, 0)
    };

    WorldBorderRenderState {
        min_x,
        max_x,
        min_z,
        max_z,
        tint,
        alpha,
        camera_position: camera,
        render_distance,
        depth_far,
        texture_offset,
    }
}

pub(crate) fn lightning_bolts_for_world(world: &WorldStore) -> Vec<LightningBoltRenderState> {
    world
        .lightning_bolt_render_sources()
        .into_iter()
        .filter_map(|source| {
            let position = [
                source.position.x as f32,
                source.position.y as f32,
                source.position.z as f32,
            ];
            position
                .into_iter()
                .all(f32::is_finite)
                .then_some(LightningBoltRenderState {
                    position,
                    seed: source.seed,
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
    let (rain, thunder) = effective_weather_levels(world);
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    let mut sky_color = rgb_u8_to_argb(
        colors
            .sky_color
            .or_else(|| dimension_sky_color_for_kind(dimension_kind))
            .unwrap_or([0, 0, 0]),
    );

    if uses_builtin_dimension_fallback && dimension_kind == VanillaLightmapDimensionKind::Overworld
    {
        sky_color = argb_multiply(
            sky_color,
            sample_periodic_argb_keyframes(
                day_time,
                &VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
        );
    }
    sky_color = apply_weather_sky_color_layers(sky_color, rain, thunder);
    if !hide_lightning_flash && world.sky_flash_time() > 0 {
        sky_color = argb_srgb_lerp(
            VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
            sky_color,
            VANILLA_SKY_FLASH_SKY_COLOR,
        );
    }
    sky_color
}

#[cfg(test)]
pub(crate) fn fog_environment_for_world_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    render_distance_chunks: u32,
    water_vision: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
) -> FogEnvironment {
    fog_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors_for_default_projection(world, camera_pose),
        render_distance_chunks,
        bbb_client_render::VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
        water_vision,
        rain_fog_multiplier,
        hide_lightning_flash,
    )
}

pub(crate) fn fog_environment_for_world_with_environment_colors(
    world: &WorldStore,
    colors: CameraEnvironmentColors,
    render_distance_chunks: u32,
    cloud_range_chunks: u32,
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
    let base = match colors.fog_type {
        CameraFogType::Atmospheric => {
            let dimension_kind = world
                .level_info()
                .map(vanilla_lightmap_dimension_kind)
                .unwrap_or(VanillaLightmapDimensionKind::Overworld);
            let (mut environmental_start, mut environmental_end) =
                atmospheric_fog_distance_for_dimension(dimension_kind);
            environmental_start = colors.fog_start_distance.unwrap_or(environmental_start);
            environmental_end = colors.fog_end_distance.unwrap_or(environmental_end);
            let mut sky_end = colors
                .sky_fog_end_distance
                .unwrap_or_else(|| atmospheric_sky_fog_end(render_distance_chunks));
            let mut cloud_end = colors
                .cloud_fog_end_distance
                .unwrap_or(VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE)
                .min(cloud_range_chunks as f32 * 16.0);
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
                colors
                    .water_fog_start_distance
                    .unwrap_or(VANILLA_DEFAULT_WATER_FOG_START_DISTANCE),
                environmental_end,
                render_distance_chunks,
                environmental_end,
                environmental_end,
            )
        }
        CameraFogType::Lava => {
            // Vanilla 26.1 `LavaFogEnvironment.setupFog`: spectator wins over
            // living-camera fire resistance, followed by the ordinary range.
            let render_distance_blocks = render_distance_chunks as f32 * 16.0;
            let (environmental_start, environmental_end) = if camera_entity_is_spectator(world) {
                (-8.0, render_distance_blocks * 0.5)
            } else if camera_entity_is_living(world)
                && camera_entity_effect(world, VANILLA_MOB_EFFECT_FIRE_RESISTANCE_ID).is_some()
            {
                (0.0, 5.0)
            } else {
                (0.25, 1.0)
            };
            FogEnvironment::world_with_visibility_ends(
                color,
                environmental_start,
                environmental_end,
                render_distance_chunks,
                environmental_end,
                environmental_end,
            )
        }
        CameraFogType::PowderSnow => {
            // Vanilla 26.1 `PowderedSnowFogEnvironment.setupFog`: the camera
            // entity's spectator state selects the wide range; otherwise the
            // environment ends two blocks from the camera.
            let render_distance_blocks = render_distance_chunks as f32 * 16.0;
            let (environmental_start, environmental_end) = if camera_entity_is_spectator(world) {
                (-8.0, render_distance_blocks * 0.5)
            } else {
                (0.0, 2.0)
            };
            FogEnvironment::world_with_visibility_ends(
                color,
                environmental_start,
                environmental_end,
                render_distance_chunks,
                environmental_end,
                environmental_end,
            )
        }
    };
    if matches!(
        colors.fog_type,
        CameraFogType::Lava | CameraFogType::PowderSnow
    ) {
        base
    } else {
        camera_mob_effect_fog_environment(
            world,
            base,
            render_distance_chunks,
            VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
        )
    }
}

pub(super) fn camera_mob_effect_fog_environment(
    world: &WorldStore,
    base: FogEnvironment,
    render_distance_chunks: u32,
    partial_tick: f32,
) -> FogEnvironment {
    let render_distance_blocks = render_distance_chunks.clamp(
        VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
    ) as f32
        * 16.0;
    if let Some(effect) = camera_entity_effect(world, VANILLA_MOB_EFFECT_BLINDNESS_ID) {
        let distance = if effect.duration_ticks == -1 {
            5.0
        } else {
            let factor = (effect.duration_ticks as f32 / 20.0).min(1.0);
            render_distance_blocks + factor * (5.0 - render_distance_blocks)
        };
        return FogEnvironment::world_with_visibility_ends(
            base.color,
            distance * 0.25,
            distance,
            render_distance_chunks,
            distance * 0.8,
            distance * 0.8,
        );
    }
    if let Some(effect) = camera_entity_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID) {
        let factor = effect.blend_factor(partial_tick);
        let distance = render_distance_blocks + factor * (15.0 - render_distance_blocks);
        return FogEnvironment::world_with_visibility_ends(
            base.color,
            distance * 0.75,
            distance,
            render_distance_chunks,
            distance,
            distance,
        );
    }
    base
}
