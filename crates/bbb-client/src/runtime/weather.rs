use super::*;

#[cfg(test)]
pub(super) const VANILLA_WEATHER_RENDER_RADIUS: u32 = 10;
const VANILLA_WEATHER_SNOW_TEMPERATURE_THRESHOLD: f32 = 0.15;
pub(super) const VANILLA_OVERWORLD_SKY_COLOR: [u8; 3] = [0x78, 0xa7, 0xff];
pub(super) const VANILLA_OVERWORLD_FOG_COLOR: [u8; 3] = [0xc0, 0xd8, 0xff];
pub(super) const VANILLA_END_FOG_COLOR: [u8; 3] = [0x18, 0x13, 0x18];
pub(super) const VANILLA_DEFAULT_WATER_FOG_COLOR: [u8; 3] = [0x05, 0x05, 0x33];
pub(super) const VANILLA_LAVA_FOG_COLOR: i32 = argb_color(255, 0x99, 0x19, 0x00);
pub(super) const VANILLA_POWDER_SNOW_FOG_COLOR: i32 = argb_color(255, 0x9f, 0xbb, 0xcc);
const VANILLA_WEATHER_RAIN_FOG_COLOR_MULTIPLIER: i32 = argb_color(255, 127, 127, 153);
const VANILLA_WEATHER_THUNDER_FOG_COLOR_MULTIPLIER: i32 = argb_color(255, 63, 63, 76);
#[cfg(test)]
pub(super) const VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS: f32 = 12.0;
const VANILLA_SUNRISE_SUNSET_MIN_RENDER_DISTANCE_CHUNKS: u32 = 4;
pub(super) const VANILLA_DEFAULT_FOG_START_DISTANCE: f32 = 0.0;
pub(super) const VANILLA_DEFAULT_FOG_END_DISTANCE: f32 = 1024.0;
pub(super) const VANILLA_NETHER_FOG_START_DISTANCE: f32 = 10.0;
pub(super) const VANILLA_NETHER_FOG_END_DISTANCE: f32 = 96.0;
pub(super) const VANILLA_DEFAULT_SKY_FOG_END_DISTANCE: f32 = 512.0;
pub(super) const VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE: f32 = 2048.0;
pub(super) const VANILLA_DEFAULT_WATER_FOG_START_DISTANCE: f32 = -8.0;
pub(super) const VANILLA_DEFAULT_WATER_FOG_END_DISTANCE: f32 = 96.0;
const VANILLA_RAIN_FOG_MIN_SKY_LIGHT: f32 = 8.0;
const VANILLA_RAIN_FOG_SKY_LIGHT_RANGE: f32 = 7.0;
const VANILLA_RAIN_FOG_START_OFFSET: f32 = -160.0;
const VANILLA_RAIN_FOG_END_OFFSET: f32 = -256.0;
pub(super) const VANILLA_RAIN_FOG_SMOOTHING_PER_TICK: f32 = 0.2;
pub(super) const VANILLA_SKY_FLASH_SKY_COLOR: i32 = argb_color(255, 204, 204, 255);
pub(super) const VANILLA_SKY_FLASH_SKY_COLOR_ALPHA: f32 = 0.22;
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct CameraEnvironmentColors {
    pub(super) sky_color: Option<[u8; 3]>,
    pub(super) fog_color: Option<[u8; 3]>,
    pub(super) sunrise_sunset_color: Option<[u8; 4]>,
    pub(super) sun_angle_degrees: Option<f32>,
    pub(super) moon_angle_degrees: Option<f32>,
    pub(super) moon_phase: Option<bbb_client_world::ClientMoonPhase>,
    pub(super) star_angle_degrees: Option<f32>,
    pub(super) star_brightness: Option<f32>,
    pub(super) fog_start_distance: Option<f32>,
    pub(super) fog_end_distance: Option<f32>,
    pub(super) sky_fog_end_distance: Option<f32>,
    pub(super) cloud_fog_end_distance: Option<f32>,
    pub(super) water_fog_color: Option<[u8; 3]>,
    pub(super) water_fog_start_distance: Option<f32>,
    pub(super) water_fog_end_distance: Option<f32>,
    pub(super) camera_forward: Option<[f32; 3]>,
    pub(super) camera_eye_y: Option<f32>,
    pub(super) fog_type: CameraFogType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum CameraFogType {
    #[default]
    Atmospheric,
    Water,
    Lava,
    PowderSnow,
}

pub(super) fn apply_weather_cloud_color_layers(
    color: i32,
    rain_level: f64,
    thunder_level: f64,
) -> i32 {
    apply_weather_gray_layers(color, rain_level, thunder_level, [0.24, 0.5], [0.095, 0.94])
}

pub(super) fn apply_weather_sky_color_layers(
    color: i32,
    rain_level: f64,
    thunder_level: f64,
) -> i32 {
    apply_weather_gray_layers(color, rain_level, thunder_level, [0.60, 0.75], [0.24, 0.94])
}

fn apply_weather_gray_layers(
    color: i32,
    rain_level: f64,
    thunder_level: f64,
    rain_gray: [f32; 2],
    thunder_gray: [f32; 2],
) -> i32 {
    let thunder_level = sanitize_weather_color_level(thunder_level);
    let rain_level = (sanitize_weather_color_level(rain_level) - thunder_level).max(0.0);
    let color = apply_weather_gray_layer(color, rain_level, rain_gray);
    apply_weather_gray_layer(color, thunder_level, thunder_gray)
}

fn apply_weather_gray_layer(color: i32, level: f32, [brightness, factor]: [f32; 2]) -> i32 {
    if level <= 0.0 {
        return color;
    }
    let modified = packed_color::blend_to_gray(color as u32, brightness, factor);
    argb_srgb_lerp(level, color, modified as i32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WeatherPrecipitation {
    Rain,
    Snow,
}

pub(super) fn world_can_have_weather(world: &WorldStore) -> bool {
    world
        .level_info()
        .is_some_and(WorldLevelInfo::can_have_weather)
}

pub(super) fn weather_precipitation_at(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    pos: BlockPos,
    sea_level: i32,
) -> Option<WeatherPrecipitation> {
    let biome_id = world.biome_id_at(pos);
    if !terrain_textures
        .biome_has_precipitation(biome_id)
        .unwrap_or(true)
    {
        return None;
    }
    let temperature = terrain_textures.biome_temperature(biome_id).unwrap_or(0.5);
    let temperature_modifier = terrain_textures
        .biome_temperature_modifier(biome_id)
        .unwrap_or_default();
    if weather_cold_enough_to_snow(temperature, temperature_modifier, pos, sea_level) {
        Some(WeatherPrecipitation::Snow)
    } else {
        Some(WeatherPrecipitation::Rain)
    }
}

fn weather_cold_enough_to_snow(
    base_temperature: f32,
    temperature_modifier: bbb_client_assets::BiomeTemperatureModifier,
    pos: BlockPos,
    sea_level: i32,
) -> bool {
    let adjusted_temperature =
        biome_height_adjusted_temperature(base_temperature, temperature_modifier, pos, sea_level);
    adjusted_temperature < VANILLA_WEATHER_SNOW_TEMPERATURE_THRESHOLD
}

pub(super) fn weather_motion_blocking_height(world: &WorldStore, x: i32, z: i32) -> Option<i32> {
    if let Some(height) = world.sample_motion_blocking_height(x, z) {
        return Some(height);
    }
    let dimension = world.dimension();
    let top_y = dimension.min_y + dimension.height - 1;
    for y in (dimension.min_y..=top_y).rev() {
        let probe = world.block_view(BlockPos { x, y, z })?;
        if weather_motion_blocking_material(probe.material) || probe.fluid.is_some() {
            return Some(y + 1);
        }
    }
    Some(dimension.min_y)
}

fn weather_motion_blocking_material(material: TerrainMaterialClass) -> bool {
    !matches!(
        material,
        TerrainMaterialClass::Empty | TerrainMaterialClass::Invisible
    )
}

pub(super) fn rain_weather_column(
    x: i32,
    z: i32,
    bottom_y: i32,
    top_y: i32,
    light: VanillaLightCoords,
    ticks: i32,
    partial_tick: f32,
) -> WeatherColumn {
    let wrapped_ticks = ticks & 131_071;
    let tick_offset = weather_column_offset_seed(x, z) & 0xff;
    let mut random = WeatherLegacyRandom::new(i64::from(weather_column_xor_seed(x, z)));
    let block_pos_rain_speed = 3.0 + random.next_float();
    let texture_offset =
        -((wrapped_ticks + tick_offset) as f32 + partial_tick) / 32.0 * block_pos_rain_speed;
    WeatherColumn::new(
        x,
        z,
        bottom_y,
        top_y,
        0.0,
        texture_offset % 32.0,
        weather_light_coords(light),
    )
}

pub(super) fn snow_weather_column(
    x: i32,
    z: i32,
    bottom_y: i32,
    top_y: i32,
    light: VanillaLightCoords,
    ticks: i32,
    partial_tick: f32,
) -> WeatherColumn {
    let time = ticks as f32 + partial_tick;
    let mut random = WeatherLegacyRandom::new(i64::from(weather_column_xor_seed(x, z)));
    let u = random.next_double() as f32 + time * 0.01 * random.next_gaussian() as f32;
    let v = random.next_double() as f32 + time * random.next_gaussian() as f32 * 0.001;
    let v_offset = -((ticks & 511) as f32 + partial_tick) / 512.0;
    WeatherColumn::new(
        x,
        z,
        bottom_y,
        top_y,
        u,
        v_offset + v,
        weather_snow_light_coords(light),
    )
}

fn weather_column_xor_seed(x: i32, z: i32) -> i32 {
    x.wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45_238_971))
        ^ z.wrapping_mul(z)
            .wrapping_mul(418_711)
            .wrapping_add(z.wrapping_mul(13_761))
}

fn weather_column_offset_seed(x: i32, z: i32) -> i32 {
    x.wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45_238_971))
        .wrapping_add(z.wrapping_mul(z).wrapping_mul(418_711))
        .wrapping_add(z.wrapping_mul(13_761))
}

fn weather_light_coords(light: VanillaLightCoords) -> [f32; 2] {
    light.normalized()
}

fn weather_snow_light_coords(light: VanillaLightCoords) -> [f32; 2] {
    [
        ((u16::from(light.block()) * 3 + 15) / 4) as f32 / 15.0,
        ((u16::from(light.sky()) * 3 + 15) / 4) as f32 / 15.0,
    ]
}

#[derive(Debug, Clone)]
struct WeatherLegacyRandom {
    seed: u64,
    next_gaussian: Option<f64>,
}

impl WeatherLegacyRandom {
    const MASK: u64 = (1u64 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;

    fn new(seed: i64) -> Self {
        let mut random = Self {
            seed: 0,
            next_gaussian: None,
        };
        random.set_seed(seed);
        random
    }

    fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK;
        self.next_gaussian = None;
    }

    fn next(&mut self, bits: u8) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1u32 << 24) as f32
    }

    fn next_double(&mut self) -> f64 {
        let upper = u64::from(self.next(26));
        let lower = u64::from(self.next(27));
        ((upper << 27) + lower) as f64 / (1u64 << 53) as f64
    }

    fn next_gaussian(&mut self) -> f64 {
        if let Some(value) = self.next_gaussian.take() {
            return value;
        }

        loop {
            let x = 2.0 * self.next_double() - 1.0;
            let y = 2.0 * self.next_double() - 1.0;
            let radius_sq = x * x + y * y;
            if radius_sq >= 1.0 || radius_sq == 0.0 {
                continue;
            }
            let multiplier = (-2.0 * radius_sq.ln() / radius_sq).sqrt();
            self.next_gaussian = Some(y * multiplier);
            return x * multiplier;
        }
    }
}

pub(super) fn atmospheric_sky_fog_end(render_distance_chunks: u32) -> f32 {
    vanilla_render_distance_blocks(render_distance_chunks).min(VANILLA_DEFAULT_SKY_FOG_END_DISTANCE)
}

fn vanilla_render_distance_blocks(render_distance_chunks: u32) -> f32 {
    render_distance_chunks.clamp(
        VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
    ) as f32
        * 16.0
}

pub(super) fn atmospheric_fog_distance_for_dimension(
    kind: VanillaLightmapDimensionKind,
) -> (f32, f32) {
    match kind {
        VanillaLightmapDimensionKind::Nether => (
            VANILLA_NETHER_FOG_START_DISTANCE,
            VANILLA_NETHER_FOG_END_DISTANCE,
        ),
        VanillaLightmapDimensionKind::Overworld
        | VanillaLightmapDimensionKind::End
        | VanillaLightmapDimensionKind::Other => (
            VANILLA_DEFAULT_FOG_START_DISTANCE,
            VANILLA_DEFAULT_FOG_END_DISTANCE,
        ),
    }
}

pub(super) fn apply_atmospheric_rain_fog_distance(
    environmental_start: &mut f32,
    environmental_end: &mut f32,
    rain_fog_multiplier: f32,
) {
    let rain_fog_multiplier = rain_fog_multiplier.clamp(0.0, 1.0);
    if rain_fog_multiplier <= 0.0 {
        return;
    }

    *environmental_start += VANILLA_RAIN_FOG_START_OFFSET * rain_fog_multiplier;
    let min_rain_fog_end = VANILLA_NETHER_FOG_END_DISTANCE.min(*environmental_end);
    *environmental_end = min_rain_fog_end
        .max(*environmental_end + VANILLA_RAIN_FOG_END_OFFSET * rain_fog_multiplier);
}

pub(super) fn atmospheric_rain_fog_target_multiplier(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
) -> f32 {
    if world.level_info().map(vanilla_lightmap_dimension_kind)
        != Some(VanillaLightmapDimensionKind::Overworld)
    {
        return 0.0;
    }

    let rain_level = sanitize_weather_lightmap_level(world.weather().rain_level);
    if rain_level <= 0.0 {
        return 0.0;
    }

    let Some(block_pos) = camera_block_position(world) else {
        return 0.0;
    };
    let sky_light = world
        .sample_block_light(block_pos)
        .map(|light| light.sky)
        .unwrap_or(15) as f32;
    let sky_light_multiplier = ((sky_light - VANILLA_RAIN_FOG_MIN_SKY_LIGHT)
        / VANILLA_RAIN_FOG_SKY_LIGHT_RANGE)
        .clamp(0.0, 1.0);
    let precipitation_multiplier =
        if camera_biome_has_precipitation(world, terrain_textures, block_pos) {
            1.0
        } else {
            0.5
        };

    rain_level * sky_light_multiplier * precipitation_multiplier
}

pub(super) fn dimension_sky_color_for_kind(kind: VanillaLightmapDimensionKind) -> Option<[u8; 3]> {
    match kind {
        VanillaLightmapDimensionKind::Overworld => Some(VANILLA_OVERWORLD_SKY_COLOR),
        VanillaLightmapDimensionKind::Nether | VanillaLightmapDimensionKind::End => Some([0, 0, 0]),
        VanillaLightmapDimensionKind::Other => None,
    }
}

pub(super) fn dimension_fog_color_for_kind(kind: VanillaLightmapDimensionKind) -> Option<[u8; 3]> {
    match kind {
        VanillaLightmapDimensionKind::Overworld => Some(VANILLA_OVERWORLD_FOG_COLOR),
        VanillaLightmapDimensionKind::End => Some(VANILLA_END_FOG_COLOR),
        VanillaLightmapDimensionKind::Nether | VanillaLightmapDimensionKind::Other => None,
    }
}

#[cfg(test)]
pub(crate) fn clear_color_for_day_time(
    day_time: i64,
    rain_level: f64,
    thunder_level: f64,
) -> ClearColor {
    clear_color_for_day_time_with_environment_colors(
        day_time,
        rain_level,
        thunder_level,
        Some(VANILLA_OVERWORLD_FOG_COLOR),
        Some(VANILLA_OVERWORLD_SKY_COLOR),
        VanillaLightmapDimensionKind::Overworld,
    )
}

#[cfg(test)]
pub(super) fn clear_color_for_day_time_with_environment_colors(
    day_time: i64,
    rain_level: f64,
    thunder_level: f64,
    fog_color: Option<[u8; 3]>,
    sky_color: Option<[u8; 3]>,
    dimension_kind: VanillaLightmapDimensionKind,
) -> ClearColor {
    clear_color_for_day_time_with_environment_colors_and_camera(
        day_time,
        rain_level,
        thunder_level,
        fog_color,
        sky_color,
        dimension_kind,
        None,
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
    )
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AtmosphericClearColorInputs {
    pub(super) day_time: i64,
    pub(super) rain_level: f64,
    pub(super) thunder_level: f64,
    pub(super) fog_color: Option<[u8; 3]>,
    pub(super) sky_color: Option<[u8; 3]>,
    pub(super) sun_angle_degrees: Option<f32>,
    pub(super) sunrise_sunset_color: Option<[u8; 4]>,
    pub(super) sky_fog_end_distance: Option<f32>,
    pub(super) dimension_kind: VanillaLightmapDimensionKind,
    pub(super) uses_builtin_dimension_fallback: bool,
    pub(super) camera_forward: Option<[f32; 3]>,
    pub(super) render_distance_chunks: u32,
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn clear_color_for_day_time_with_environment_colors_and_camera(
    day_time: i64,
    rain_level: f64,
    thunder_level: f64,
    fog_color: Option<[u8; 3]>,
    sky_color: Option<[u8; 3]>,
    dimension_kind: VanillaLightmapDimensionKind,
    camera_forward: Option<[f32; 3]>,
    render_distance_chunks: u32,
) -> ClearColor {
    clear_color_for_atmospheric_environment(AtmosphericClearColorInputs {
        day_time,
        rain_level,
        thunder_level,
        fog_color,
        sky_color,
        sun_angle_degrees: None,
        sunrise_sunset_color: None,
        sky_fog_end_distance: None,
        dimension_kind,
        uses_builtin_dimension_fallback: true,
        camera_forward,
        render_distance_chunks,
    })
}

/// Vanilla `AtmosphericFogEnvironment.getBaseColor` (`AtmosphericFogEnvironment.java:26-48`).
pub(super) fn clear_color_for_atmospheric_environment(
    inputs: AtmosphericClearColorInputs,
) -> ClearColor {
    let mut fog_color = rgb_u8_to_argb(inputs.fog_color.unwrap_or([0, 0, 0]));
    let mut sky_color = rgb_u8_to_argb(inputs.sky_color.unwrap_or([0, 0, 0]));
    // Synced 26.1 dimensions already evaluate day/night timeline multipliers in
    // `EnvironmentAttributes`; apply built-in overworld curves only when no
    // synchronized `minecraft:dimension_type` registry is present.
    if inputs.uses_builtin_dimension_fallback
        && inputs.dimension_kind == VanillaLightmapDimensionKind::Overworld
    {
        fog_color = argb_multiply(
            fog_color,
            sample_periodic_argb_keyframes(
                inputs.day_time,
                &VANILLA_OVERWORLD_FOG_COLOR_MULTIPLIER_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
        );
        sky_color = argb_multiply(
            sky_color,
            sample_periodic_argb_keyframes(
                inputs.day_time,
                &VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
        );
    }
    fog_color = apply_weather_fog_color_layers(fog_color, inputs.rain_level, inputs.thunder_level);
    fog_color = apply_sunrise_sunset_fog_color(fog_color, inputs);
    sky_color = apply_weather_sky_color_layers(sky_color, inputs.rain_level, inputs.thunder_level);
    sky_color =
        apply_atmospheric_sky_weather_darken(sky_color, inputs.rain_level, inputs.thunder_level);
    atmospheric_clear_color_with_sky_fog_end(
        fog_color,
        sky_color,
        inputs.sky_fog_end_distance,
        inputs.render_distance_chunks,
    )
}

fn apply_weather_fog_color_layers(color: i32, rain_level: f64, thunder_level: f64) -> i32 {
    let thunder_level = sanitize_weather_color_level(thunder_level);
    let rain_level = (sanitize_weather_color_level(rain_level) - thunder_level).max(0.0);
    let color =
        apply_weather_fog_color_layer(color, rain_level, VANILLA_WEATHER_RAIN_FOG_COLOR_MULTIPLIER);
    apply_weather_fog_color_layer(
        color,
        thunder_level,
        VANILLA_WEATHER_THUNDER_FOG_COLOR_MULTIPLIER,
    )
}

fn apply_weather_fog_color_layer(color: i32, level: f32, multiplier: i32) -> i32 {
    if level <= 0.0 {
        return color;
    }
    argb_srgb_lerp(level, color, argb_multiply(color, multiplier))
}

fn apply_sunrise_sunset_fog_color(fog_color: i32, inputs: AtmosphericClearColorInputs) -> i32 {
    if inputs.render_distance_chunks < VANILLA_SUNRISE_SUNSET_MIN_RENDER_DISTANCE_CHUNKS
        || (inputs.uses_builtin_dimension_fallback
            && inputs.dimension_kind != VanillaLightmapDimensionKind::Overworld)
    {
        return fog_color;
    }

    let Some(camera_forward) = inputs.camera_forward else {
        return fog_color;
    };
    let sun_angle = inputs
        .sun_angle_degrees
        .unwrap_or_else(|| {
            if inputs.uses_builtin_dimension_fallback {
                overworld_sun_angle(inputs.day_time)
            } else {
                0.0
            }
        })
        .to_radians();
    let sun_x = if sun_angle.sin() > 0.0 { -1.0 } else { 1.0 };
    let looking_at_sun = camera_forward[0] * sun_x;
    if looking_at_sun <= 0.0 {
        return fog_color;
    }

    let mut sunrise_color = inputs
        .sunrise_sunset_color
        .map(|[red, green, blue, alpha]| {
            argb_color(
                i32::from(alpha),
                i32::from(red),
                i32::from(green),
                i32::from(blue),
            )
        })
        .unwrap_or_else(|| {
            if inputs.uses_builtin_dimension_fallback {
                sample_periodic_argb_keyframes(
                    inputs.day_time,
                    &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
                    VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
                )
            } else {
                0
            }
        });
    sunrise_color = apply_weather_sunrise_sunset_color_layers(
        sunrise_color,
        inputs.rain_level,
        inputs.thunder_level,
    );
    let alpha = argb_alpha(sunrise_color) as f32 / 255.0;
    if alpha <= 0.0 {
        return fog_color;
    }

    argb_srgb_lerp(
        looking_at_sun * alpha,
        fog_color,
        argb_opaque(sunrise_color),
    )
}

pub(super) fn apply_weather_sunrise_sunset_color_layers(
    color: i32,
    rain_level: f64,
    thunder_level: f64,
) -> i32 {
    apply_weather_fog_color_layers(color, rain_level, thunder_level)
}

pub(super) fn overworld_sun_angle(day_time: i64) -> f32 {
    ((day_time - VANILLA_LIGHTMAP_DEFAULT_DAY_TIME) as f32 * 360.0
        / VANILLA_LIGHTMAP_DAY_PERIOD_TICKS as f32)
        .rem_euclid(360.0)
}

pub(super) fn overworld_moon_angle(day_time: i64) -> f32 {
    (overworld_sun_angle(day_time) + 180.0).rem_euclid(360.0)
}

pub(super) fn overworld_star_angle(day_time: i64) -> f32 {
    overworld_sun_angle(day_time)
}

pub(super) fn overworld_moon_phase(day_time: i64) -> SkyMoonPhase {
    let phase = day_time
        .rem_euclid(VANILLA_LIGHTMAP_DAY_PERIOD_TICKS * SkyMoonPhase::ALL.len() as i64)
        / VANILLA_LIGHTMAP_DAY_PERIOD_TICKS;
    SkyMoonPhase::from_vanilla_index(phase as usize)
}

pub(super) fn client_moon_phase_index(phase: bbb_client_world::ClientMoonPhase) -> u8 {
    match phase {
        bbb_client_world::ClientMoonPhase::FullMoon => 0,
        bbb_client_world::ClientMoonPhase::WaningGibbous => 1,
        bbb_client_world::ClientMoonPhase::ThirdQuarter => 2,
        bbb_client_world::ClientMoonPhase::WaningCrescent => 3,
        bbb_client_world::ClientMoonPhase::NewMoon => 4,
        bbb_client_world::ClientMoonPhase::WaxingCrescent => 5,
        bbb_client_world::ClientMoonPhase::FirstQuarter => 6,
        bbb_client_world::ClientMoonPhase::WaxingGibbous => 7,
    }
}

pub(super) fn apply_weather_star_brightness_layers(
    brightness: f32,
    rain_level: f32,
    thunder_level: f32,
) -> f32 {
    let thunder_level = thunder_level.clamp(0.0, 1.0);
    let rain_without_thunder = (rain_level.clamp(0.0, 1.0) - thunder_level).max(0.0);
    brightness * (1.0 - rain_without_thunder) * (1.0 - thunder_level)
}

fn apply_atmospheric_sky_weather_darken(color: i32, rain_level: f64, thunder_level: f64) -> i32 {
    let rain_level = sanitize_weather_color_level(rain_level);
    let thunder_level = sanitize_weather_color_level(thunder_level);
    let mut color = color;
    if rain_level > 0.0 {
        color = argb_scale_rgb(
            color,
            1.0 - rain_level * 0.5,
            1.0 - rain_level * 0.5,
            1.0 - rain_level * 0.4,
        );
    }
    if thunder_level > 0.0 {
        color = argb_scale_rgb(
            color,
            1.0 - thunder_level * 0.5,
            1.0 - thunder_level * 0.5,
            1.0 - thunder_level * 0.5,
        );
    }
    color
}

pub(super) fn apply_fog_brightening(color: i32, brighten_factor: f32) -> i32 {
    if argb_red(color) == 0 || argb_green(color) == 0 || argb_blue(color) == 0 {
        return color;
    }
    let brighten_factor = brighten_factor.clamp(0.0, 1.0);
    if brighten_factor <= 0.0 {
        return color;
    }

    let red = argb_red(color) as f32 / 255.0;
    let green = argb_green(color) as f32 / 255.0;
    let blue = argb_blue(color) as f32 / 255.0;
    let target_scale = 1.0 / red.max(green).max(blue);
    argb_color(
        argb_alpha(color),
        argb_channel_from_unit(red + (red * target_scale - red) * brighten_factor),
        argb_channel_from_unit(green + (green * target_scale - green) * brighten_factor),
        argb_channel_from_unit(blue + (blue * target_scale - blue) * brighten_factor),
    )
}

#[cfg(test)]
pub(super) fn atmospheric_clear_color(
    fog_color: i32,
    sky_color: i32,
    render_distance_chunks: u32,
) -> ClearColor {
    atmospheric_clear_color_with_sky_fog_end(fog_color, sky_color, None, render_distance_chunks)
}

fn atmospheric_clear_color_with_sky_fog_end(
    fog_color: i32,
    sky_color: i32,
    sky_fog_end_distance: Option<f32>,
    render_distance_chunks: u32,
) -> ClearColor {
    let sky_fog_end_blocks = sky_fog_end_distance
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(VANILLA_DEFAULT_SKY_FOG_END_DISTANCE);
    let sky_fog_end = (sky_fog_end_blocks / 16.0).min(render_distance_chunks as f32);
    let sky_color_mix = clamped_lerp(sky_fog_end / 32.0, 0.25, 1.0);
    let sky_color_mix = 1.0 - sky_color_mix.powf(0.25);
    clear_color_from_argb(argb_srgb_lerp(sky_color_mix, fog_color, sky_color))
}

fn sanitize_weather_color_level(level: f64) -> f32 {
    if level.is_finite() {
        level.clamp(0.0, 1.0) as f32
    } else {
        0.0
    }
}

fn clamped_lerp(factor: f32, min: f32, max: f32) -> f32 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        min + factor * (max - min)
    }
}

pub(super) fn argb_scale_rgb(color: i32, red: f32, green: f32, blue: f32) -> i32 {
    packed_color::scale_rgb_channels(color as u32, red, green, blue) as i32
}

#[cfg(test)]
fn camera_environment_colors(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    camera_fluid: CameraFluid,
) -> CameraEnvironmentColors {
    let attributes = sampled_camera_fog_and_sky_attributes_at_camera(world, camera_pose);
    camera_environment_colors_from_attributes(&attributes, camera_pose, camera_fluid)
}

pub(super) fn camera_environment_colors_from_cached_sample(
    samples: &mut CameraEnvironmentSampleCache,
    world: &WorldStore,
    level_generation: u64,
    client_tick_index: u64,
    partial_tick: f32,
    camera_pose: Option<CameraPose>,
    camera_fluid: CameraFluid,
) -> CameraEnvironmentColors {
    let attributes = samples.sample(
        world,
        level_generation,
        client_tick_index,
        partial_tick,
        camera_pose,
    );
    camera_environment_colors_from_attributes(&attributes, camera_pose, camera_fluid)
}

fn camera_environment_colors_from_attributes(
    attributes: &ClientEnvironmentAttributes,
    camera_pose: Option<CameraPose>,
    camera_fluid: CameraFluid,
) -> CameraEnvironmentColors {
    CameraEnvironmentColors {
        sky_color: attributes.sky_color,
        fog_color: attributes.fog_color,
        sunrise_sunset_color: attributes.sunrise_sunset_color,
        sun_angle_degrees: attributes.sun_angle_degrees,
        moon_angle_degrees: attributes.moon_angle_degrees,
        moon_phase: attributes.moon_phase,
        star_angle_degrees: attributes.star_angle_degrees,
        star_brightness: attributes.star_brightness,
        fog_start_distance: attributes.fog_start_distance,
        fog_end_distance: attributes.fog_end_distance,
        sky_fog_end_distance: attributes.sky_fog_end_distance,
        cloud_fog_end_distance: attributes.cloud_fog_end_distance,
        water_fog_color: attributes.water_fog_color,
        water_fog_start_distance: attributes.water_fog_start_distance,
        water_fog_end_distance: attributes.water_fog_end_distance,
        camera_forward: camera_pose.map(camera_forward_vector),
        camera_eye_y: camera_pose.map(|camera| camera.position[1] + camera.eye_height),
        fog_type: match camera_fluid {
            CameraFluid::None => CameraFogType::Atmospheric,
            CameraFluid::Water => CameraFogType::Water,
            CameraFluid::Lava => CameraFogType::Lava,
            CameraFluid::PowderSnow => CameraFogType::PowderSnow,
        },
    }
}

#[cfg(test)]
pub(super) fn camera_environment_colors_for_default_projection(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> CameraEnvironmentColors {
    let camera_fluid = camera_pose.map_or(CameraFluid::None, |camera| {
        camera_fluid_in_camera(
            world,
            camera,
            [1920, 1080],
            crate::camera_fov::VANILLA_DEFAULT_CAMERA_FOV_DEGREES,
        )
    });
    camera_environment_colors(world, camera_pose, camera_fluid)
}

pub(super) fn sampled_camera_fog_and_sky_attributes_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> ClientEnvironmentAttributes {
    let sampled = camera_pose.map(camera_eye_position).map_or_else(
        || world.sampled_client_environment_attributes(),
        |eye| world.sampled_client_camera_environment_attributes_at(eye.map(f64::from)),
    );
    sampled.unwrap_or_else(|_| {
        world
            .level_info()
            .map(WorldLevelInfo::environment_attributes)
            .unwrap_or_default()
    })
}

pub(super) fn sampled_full_environment_attributes_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> ClientEnvironmentAttributes {
    let sampled = camera_pose.map(camera_eye_position).map_or_else(
        || world.sampled_client_environment_attributes(),
        |eye| {
            world.sampled_client_environment_attributes_at(
                eye.map(f64::from),
                EnvironmentBiomeSampling::Gaussian,
            )
        },
    );
    sampled.unwrap_or_else(|_| {
        world
            .level_info()
            .map(WorldLevelInfo::environment_attributes)
            .unwrap_or_default()
    })
}

pub(super) fn camera_block_position(world: &WorldStore) -> Option<BlockPos> {
    let eye = camera_eye_position(camera_pose_from_world(world)?);
    if !eye.into_iter().all(f32::is_finite) {
        return None;
    }
    Some(BlockPos {
        x: eye[0].floor() as i32,
        y: eye[1].floor() as i32,
        z: eye[2].floor() as i32,
    })
}

fn camera_biome_has_precipitation(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    block_pos: BlockPos,
) -> bool {
    let biome_id = world.biome_id_at(block_pos);
    terrain_textures
        .biome_has_precipitation(biome_id)
        .unwrap_or(true)
}

pub(super) fn rgb_u8_to_argb(color: [u8; 3]) -> i32 {
    argb_color(
        255,
        i32::from(color[0]),
        i32::from(color[1]),
        i32::from(color[2]),
    )
}

pub(super) fn clear_color_with_sky_flash(clear: ClearColor) -> ClearColor {
    clear_color_from_argb(argb_srgb_lerp(
        VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
        clear_color_to_argb(clear),
        VANILLA_SKY_FLASH_SKY_COLOR,
    ))
}

pub(super) fn clear_color_to_argb(clear: ClearColor) -> i32 {
    argb_color(
        255,
        argb_channel_from_unit(clear.r as f32),
        argb_channel_from_unit(clear.g as f32),
        argb_channel_from_unit(clear.b as f32),
    )
}

pub(super) fn clear_color_from_argb(color: i32) -> ClearColor {
    ClearColor {
        r: argb_red(color) as f64 / 255.0,
        g: argb_green(color) as f64 / 255.0,
        b: argb_blue(color) as f64 / 255.0,
        a: argb_alpha(color) as f64 / 255.0,
    }
}

pub(super) fn terrain_camera_position(world: &WorldStore) -> [f32; 3] {
    camera_pose_from_world(world)
        .map(|pose| {
            [
                pose.position[0],
                pose.position[1] + pose.eye_height,
                pose.position[2],
            ]
        })
        .unwrap_or([0.0, 0.0, 0.0])
}
