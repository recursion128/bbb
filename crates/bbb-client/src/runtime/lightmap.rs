use super::*;

const VANILLA_OVERWORLD_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-16_119_286);
const VANILLA_NETHER_SKY_LIGHT_COLOR: [f32; 3] = [122.0 / 255.0, 122.0 / 255.0, 1.0];
const VANILLA_NETHER_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-13_621_215);
const VANILLA_END_SKY_LIGHT_COLOR: [f32; 3] = rgb24(-5_480_243);
const VANILLA_END_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-12_630_209);
pub(super) const VANILLA_LIGHTMAP_DEFAULT_DAY_TIME: i64 = 6_000;
const VANILLA_WEATHER_SKY_LIGHT_FACTOR: f32 = 0.24;
const VANILLA_WEATHER_RAIN_ALPHA: f32 = 0.3125;
const VANILLA_WEATHER_THUNDER_ALPHA: f32 = 0.52734375;
const VANILLA_WEATHER_RAIN_SKY_LIGHT_COLOR: i32 = argb_color(79, 122, 122, 255);
const VANILLA_WEATHER_THUNDER_SKY_LIGHT_COLOR: i32 = argb_color(134, 122, 122, 255);
const VANILLA_END_FLASH_INTERVAL_TICKS: i64 = 600;
const VANILLA_END_FLASH_MAX_OFFSET_TICKS: i32 = 200;
const VANILLA_END_FLASH_MIN_DURATION_TICKS: i32 = 100;
const VANILLA_END_FLASH_MAX_DURATION_TICKS: i32 = 380;
const VANILLA_BOSS_OVERLAY_DARKEN_IN_STEP: f32 = 0.05;
const VANILLA_BOSS_OVERLAY_DARKEN_OUT_STEP: f32 = 0.0125;
pub(super) const VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK: f32 = 1.0;
const VANILLA_NIGHT_VISION_FULL_STRENGTH_TICKS: i32 = 200;
const VANILLA_DARKNESS_EFFECT_SCALE_OPTION: f32 = 1.0;
const VANILLA_WATER_VISION_MAX_TICKS: i32 = 600;
const VANILLA_WATER_VISION_FAST_RAMP_TICKS: f32 = 100.0;
const VANILLA_WATER_VISION_SLOW_RAMP_TICKS: f32 = 500.0;
const VANILLA_WATER_VISION_FAST_WEIGHT: f32 = 0.6;
const VANILLA_WATER_VISION_SLOW_WEIGHT: f32 = 0.39999998;
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct CameraSpinningEffectTickState {
    time_degrees: f32,
    speed_degrees_per_tick: f32,
}

impl CameraSpinningEffectTickState {
    pub(super) fn advance_for_world(&mut self, ticks: u32, world: &WorldStore) {
        for _ in 0..ticks {
            let portal_intensity = world.local_player().portal_effect_intensity;
            let nausea_intensity = local_player_effect(world, VANILLA_MOB_EFFECT_NAUSEA_ID)
                .map(|effect| effect.blend_factor(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK))
                .unwrap_or(0.0);
            if portal_intensity > 0.0 || nausea_intensity > 0.0 {
                // Vanilla GameRenderer.tick weights the spin speed by both
                // sources: `(portal * 20 + nausea * 7) / (portal + nausea)`.
                self.speed_degrees_per_tick = (portal_intensity * 20.0 + nausea_intensity * 7.0)
                    / (portal_intensity + nausea_intensity);
                self.time_degrees += self.speed_degrees_per_tick;
            } else {
                self.speed_degrees_per_tick = 0.0;
            }
        }
    }

    pub(super) fn effect_for_world(
        self,
        world: &WorldStore,
        partial_tick: f32,
    ) -> Option<bbb_client_render::CameraSpinningEffect> {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        let player = world.local_player();
        let portal_intensity = player.previous_portal_effect_intensity
            + (player.portal_effect_intensity - player.previous_portal_effect_intensity)
                * partial_tick;
        let nausea_intensity = local_player_effect(world, VANILLA_MOB_EFFECT_NAUSEA_ID)
            .map(|effect| effect.blend_factor(partial_tick))
            .unwrap_or(0.0);
        let intensity = portal_intensity.max(nausea_intensity);
        (intensity > 0.0).then_some(bbb_client_render::CameraSpinningEffect {
            intensity,
            angle_degrees: self.time_degrees + partial_tick * self.speed_degrees_per_tick,
        })
    }
}

#[derive(Debug)]
pub(crate) struct LightmapTickState {
    random: LevelEventSoundRandomState,
    block_light_flicker: f32,
    brightness_factor: f32,
    client_tick_count: u64,
    water_vision_time: i32,
    end_flash_state: EndFlashLightmapState,
    boss_overlay_world_darkening: f32,
    boss_overlay_world_darkening_previous_frame: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
}

impl Default for LightmapTickState {
    fn default() -> Self {
        Self {
            random: LevelEventSoundRandomState::default(),
            block_light_flicker: 0.0,
            brightness_factor: VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
            client_tick_count: 0,
            water_vision_time: 0,
            end_flash_state: EndFlashLightmapState::default(),
            boss_overlay_world_darkening: 0.0,
            boss_overlay_world_darkening_previous_frame: 0.0,
            rain_fog_multiplier: 0.0,
            hide_lightning_flash: false,
        }
    }
}

impl LightmapTickState {
    pub(crate) fn client_tick_count(&self) -> u64 {
        self.client_tick_count
    }

    pub(crate) fn with_brightness_factor_and_hide_lightning_flash(
        brightness_factor: f32,
        hide_lightning_flash: bool,
    ) -> Self {
        Self {
            brightness_factor: sanitize_lightmap_brightness_factor(brightness_factor),
            hide_lightning_flash,
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(super) fn with_seed(seed: i64) -> Self {
        Self {
            random: LevelEventSoundRandomState::with_seed(seed),
            block_light_flicker: 0.0,
            brightness_factor: VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
            client_tick_count: 0,
            water_vision_time: 0,
            end_flash_state: EndFlashLightmapState::default(),
            boss_overlay_world_darkening: 0.0,
            boss_overlay_world_darkening_previous_frame: 0.0,
            rain_fog_multiplier: 0.0,
            hide_lightning_flash: false,
        }
    }

    #[cfg(test)]
    pub(super) fn with_seed_and_brightness(seed: i64, brightness_factor: f32) -> Self {
        Self {
            random: LevelEventSoundRandomState::with_seed(seed),
            block_light_flicker: 0.0,
            brightness_factor: sanitize_lightmap_brightness_factor(brightness_factor),
            client_tick_count: 0,
            water_vision_time: 0,
            end_flash_state: EndFlashLightmapState::default(),
            boss_overlay_world_darkening: 0.0,
            boss_overlay_world_darkening_previous_frame: 0.0,
            rain_fog_multiplier: 0.0,
            hide_lightning_flash: false,
        }
    }

    #[cfg(test)]
    pub(super) fn advance(&mut self, ticks: u32) -> f32 {
        for _ in 0..ticks {
            self.tick();
        }
        self.client_tick_count = self.client_tick_count.saturating_add(ticks as u64);
        self.block_factor()
    }

    pub(super) fn advance_for_world(&mut self, ticks: u32, world: &WorldStore) -> f32 {
        for _ in 0..ticks {
            self.tick();
            self.tick_water_vision(world);
            self.tick_end_flash(world);
            self.tick_boss_overlay_world_darkening(world);
        }
        self.client_tick_count = self.client_tick_count.saturating_add(ticks as u64);
        self.block_factor()
    }

    pub(super) fn advance_rain_fog_for_world(
        &mut self,
        ticks: u32,
        world: &WorldStore,
        terrain_textures: &TerrainTextureState,
    ) {
        for _ in 0..ticks {
            self.tick_rain_fog(world, terrain_textures);
        }
    }

    pub(super) fn environment_for_world(&self, world: &WorldStore) -> LightmapEnvironment {
        lightmap_environment_for_world_with_effects(
            world,
            crate::runtime::render_extract::LightmapEffectInputs {
                brightness_factor: self.brightness_factor,
                block_factor: self.block_light_flicker + VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR,
                camera_tick_count: self.client_tick_count,
                partial_tick: VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
                darkness_effect_factor: local_player_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID)
                    .map(|effect| effect.blend_factor(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK))
                    .unwrap_or(0.0),
                night_vision_effect: local_player_effect(world, VANILLA_MOB_EFFECT_NIGHT_VISION_ID),
                conduit_power_effect: local_player_effect(
                    world,
                    VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
                ),
                water_vision: self.water_vision(world),
                end_flash_sky_factor: self.end_flash_sky_factor(world),
                boss_overlay_world_darkening: self
                    .boss_overlay_world_darkening(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK),
                sky_flash_visible: self.sky_flash_visible(world),
            },
        )
    }

    pub(super) fn tick(&mut self) {
        // Vanilla `LightmapRenderStateExtractor.tick`: the random source is a
        // LegacyRandomSource from `RandomSource.create()`.
        let delta = (self.random.next_float() - self.random.next_float())
            * self.random.next_float()
            * self.random.next_float()
            * 0.1;
        self.block_light_flicker = (self.block_light_flicker + delta) * 0.9;
    }

    pub(super) fn block_factor(&self) -> f32 {
        self.block_light_flicker + VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR
    }

    fn tick_water_vision(&mut self, world: &WorldStore) {
        if world.local_player_eye_in_water() {
            let speed = if world.local_player_is_spectator() {
                10
            } else {
                1
            };
            self.water_vision_time =
                (self.water_vision_time + speed).clamp(0, VANILLA_WATER_VISION_MAX_TICKS);
        } else if self.water_vision_time > 0 {
            self.water_vision_time =
                (self.water_vision_time - 10).clamp(0, VANILLA_WATER_VISION_MAX_TICKS);
        }
    }

    pub(super) fn water_vision(&self, world: &WorldStore) -> f32 {
        if !world.local_player_eye_in_water() {
            return 0.0;
        }
        if self.water_vision_time >= VANILLA_WATER_VISION_MAX_TICKS {
            return 1.0;
        }

        let water_vision_time = self.water_vision_time as f32;
        let fast = (water_vision_time / VANILLA_WATER_VISION_FAST_RAMP_TICKS).clamp(0.0, 1.0);
        let slow = if self.water_vision_time < VANILLA_WATER_VISION_FAST_RAMP_TICKS as i32 {
            0.0
        } else {
            ((water_vision_time - VANILLA_WATER_VISION_FAST_RAMP_TICKS)
                / VANILLA_WATER_VISION_SLOW_RAMP_TICKS)
                .clamp(0.0, 1.0)
        };
        fast * VANILLA_WATER_VISION_FAST_WEIGHT + slow * VANILLA_WATER_VISION_SLOW_WEIGHT
    }

    fn tick_end_flash(&mut self, world: &WorldStore) {
        if world.level_info().map(vanilla_lightmap_dimension_kind)
            != Some(VanillaLightmapDimensionKind::End)
        {
            self.end_flash_state = EndFlashLightmapState::default();
            return;
        }

        self.end_flash_state
            .tick(world.world_time().map(|time| time.day_time).unwrap_or(0));
    }

    pub(super) fn end_flash_sky_factor(&self, world: &WorldStore) -> f32 {
        if self.hide_lightning_flash {
            return 0.0;
        }

        if world.level_info().map(vanilla_lightmap_dimension_kind)
            == Some(VanillaLightmapDimensionKind::End)
        {
            let intensity = self
                .end_flash_state
                .intensity(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK)
                .clamp(0.0, 1.0);
            if world.boss_overlay_should_create_world_fog() {
                intensity / 3.0
            } else {
                intensity
            }
        } else {
            0.0
        }
    }

    pub(super) fn sky_flash_visible(&self, world: &WorldStore) -> bool {
        !self.hide_lightning_flash && world.sky_flash_time() > 0
    }

    fn tick_boss_overlay_world_darkening(&mut self, world: &WorldStore) {
        self.boss_overlay_world_darkening_previous_frame = self.boss_overlay_world_darkening;
        if world.boss_overlay_should_darken_screen() {
            self.boss_overlay_world_darkening += VANILLA_BOSS_OVERLAY_DARKEN_IN_STEP;
            if self.boss_overlay_world_darkening > 1.0 {
                self.boss_overlay_world_darkening = 1.0;
            }
        } else if self.boss_overlay_world_darkening > 0.0 {
            self.boss_overlay_world_darkening -= VANILLA_BOSS_OVERLAY_DARKEN_OUT_STEP;
        }
    }

    pub(super) fn boss_overlay_world_darkening(&self, partial_tick: f32) -> f32 {
        let partial_tick = sanitize_lightmap_partial_tick(partial_tick);
        self.boss_overlay_world_darkening_previous_frame
            + (self.boss_overlay_world_darkening - self.boss_overlay_world_darkening_previous_frame)
                * partial_tick
    }

    fn tick_rain_fog(&mut self, world: &WorldStore, terrain_textures: &TerrainTextureState) {
        let target = atmospheric_rain_fog_target_multiplier(world, terrain_textures);
        self.rain_fog_multiplier +=
            (target - self.rain_fog_multiplier) * VANILLA_RAIN_FOG_SMOOTHING_PER_TICK;
    }

    pub(super) fn rain_fog_multiplier(&self) -> f32 {
        self.rain_fog_multiplier.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct EndFlashLightmapState {
    flash_seed: i64,
    offset: i32,
    duration: i32,
    intensity: f32,
    old_intensity: f32,
}

impl EndFlashLightmapState {
    fn tick(&mut self, clock_time: i64) {
        self.calculate_flash_parameters(clock_time);
        self.old_intensity = self.intensity;
        self.intensity = self.calculate_intensity(clock_time);
    }

    fn calculate_flash_parameters(&mut self, clock_time: i64) {
        let new_seed = clock_time / VANILLA_END_FLASH_INTERVAL_TICKS;
        if new_seed == self.flash_seed {
            return;
        }

        let mut random = LevelEventSoundRandomState::with_seed(new_seed);
        random.next_float();
        self.offset = random_between_inclusive(&mut random, 0, VANILLA_END_FLASH_MAX_OFFSET_TICKS);
        let max_duration = VANILLA_END_FLASH_MAX_DURATION_TICKS.min(600 - self.offset);
        self.duration = random_between_inclusive(
            &mut random,
            VANILLA_END_FLASH_MIN_DURATION_TICKS,
            max_duration,
        );
        self.flash_seed = new_seed;
    }

    fn calculate_intensity(&self, clock_time: i64) -> f32 {
        if self.duration <= 0 {
            return 0.0;
        }

        let clock_time_within_interval = clock_time % VANILLA_END_FLASH_INTERVAL_TICKS;
        let offset = i64::from(self.offset);
        let duration = i64::from(self.duration);
        if clock_time_within_interval >= offset && clock_time_within_interval <= offset + duration {
            ((clock_time_within_interval - offset) as f32 * std::f32::consts::PI
                / self.duration as f32)
                .sin()
        } else {
            0.0
        }
    }

    fn intensity(&self, partial_tick: f32) -> f32 {
        let partial_tick = sanitize_lightmap_partial_tick(partial_tick);
        self.old_intensity + (self.intensity - self.old_intensity) * partial_tick
    }
}

fn sanitize_lightmap_brightness_factor(factor: f32) -> f32 {
    if factor.is_finite() {
        factor.clamp(0.0, 1.0)
    } else {
        VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LocalPlayerLightmapEffects {
    pub(super) brightness_factor: f32,
    pub(super) darkness_scale: f32,
    pub(super) night_vision_factor: f32,
}

pub(super) fn local_player_lightmap_effects(
    brightness_factor: f32,
    camera_tick_count: u64,
    partial_tick: f32,
    darkness_effect_factor: f32,
    night_vision_effect: Option<MobEffectState>,
    conduit_power_effect: Option<MobEffectState>,
    water_vision: f32,
) -> LocalPlayerLightmapEffects {
    let partial_tick = sanitize_lightmap_partial_tick(partial_tick);
    let base_brightness = sanitize_lightmap_brightness_factor(brightness_factor);
    let darkness_modifier =
        darkness_effect_factor.clamp(0.0, 1.0) * VANILLA_DARKNESS_EFFECT_SCALE_OPTION;
    let brightness_factor = (base_brightness - darkness_modifier).max(0.0);
    let darkness_scale = vanilla_darkness_scale(camera_tick_count, darkness_modifier, partial_tick)
        * VANILLA_DARKNESS_EFFECT_SCALE_OPTION;
    let water_vision = water_vision.clamp(0.0, 1.0);
    let night_vision_factor = if let Some(effect) = night_vision_effect {
        vanilla_night_vision_scale(effect, partial_tick)
    } else if water_vision > 0.0 && conduit_power_effect.is_some() {
        water_vision
    } else {
        0.0
    };

    LocalPlayerLightmapEffects {
        brightness_factor,
        darkness_scale,
        night_vision_factor,
    }
}

pub(super) fn vanilla_night_vision_scale(effect: MobEffectState, partial_tick: f32) -> f32 {
    if !mob_effect_ends_within(effect, VANILLA_NIGHT_VISION_FULL_STRENGTH_TICKS) {
        1.0
    } else {
        0.7 + ((effect.duration_ticks as f32 - partial_tick) * std::f32::consts::PI * 0.2).sin()
            * 0.3
    }
}

fn vanilla_darkness_scale(camera_tick_count: u64, darkness_gamma: f32, partial_tick: f32) -> f32 {
    let darkness = 0.45 * darkness_gamma;
    (((camera_tick_count as f32 - partial_tick) * std::f32::consts::PI * 0.025).cos() * darkness)
        .max(0.0)
}

pub(super) fn mob_effect_ends_within(effect: MobEffectState, ticks: i32) -> bool {
    effect.duration_ticks != -1 && effect.duration_ticks <= ticks
}

fn sanitize_lightmap_partial_tick(partial_tick: f32) -> f32 {
    if partial_tick.is_finite() {
        partial_tick.clamp(0.0, 1.0)
    } else {
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK
    }
}

pub(super) fn apply_overworld_timeline_lightmap_environment(
    environment: &mut LightmapEnvironment,
    day_time: i64,
) {
    let sky_factor = sample_periodic_float_keyframes(
        day_time,
        &VANILLA_OVERWORLD_SKY_LIGHT_FACTOR_KEYFRAMES,
        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
    );
    let sky_light_color = sample_periodic_argb_keyframes(
        day_time,
        &VANILLA_OVERWORLD_SKY_LIGHT_COLOR_KEYFRAMES,
        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
    );
    environment.sky_factor *= sky_factor;
    environment.sky_light_color = rgb24(argb_multiply(
        rgb01_to_argb(environment.sky_light_color),
        sky_light_color,
    ));
}

pub(super) fn apply_weather_lightmap_environment(
    environment: &mut LightmapEnvironment,
    weather: WorldWeatherState,
) {
    let rain_level = sanitize_weather_lightmap_level(weather.rain_level);
    let thunder_level =
        (sanitize_weather_lightmap_level(weather.thunder_level) * rain_level).clamp(0.0, 1.0);
    let rain_level = (rain_level - thunder_level).max(0.0);

    apply_weather_lightmap_layer(
        environment,
        rain_level,
        VANILLA_WEATHER_RAIN_ALPHA,
        VANILLA_WEATHER_RAIN_SKY_LIGHT_COLOR,
    );
    apply_weather_lightmap_layer(
        environment,
        thunder_level,
        VANILLA_WEATHER_THUNDER_ALPHA,
        VANILLA_WEATHER_THUNDER_SKY_LIGHT_COLOR,
    );
}

fn apply_weather_lightmap_layer(
    environment: &mut LightmapEnvironment,
    level: f32,
    alpha: f32,
    sky_light_color: i32,
) {
    if level <= 0.0 {
        return;
    }

    let modified_sky_factor = environment.sky_factor
        + alpha * (VANILLA_WEATHER_SKY_LIGHT_FACTOR - environment.sky_factor);
    environment.sky_factor += level * (modified_sky_factor - environment.sky_factor);

    let base_color = rgb01_to_argb(environment.sky_light_color);
    let modified_color = argb_alpha_blend(base_color, sky_light_color);
    environment.sky_light_color = rgb24(argb_srgb_lerp(level, base_color, modified_color));
}

/// Vanilla `Level.skyDarken`, sourced from the gameplay sky-light-level attribute rather than
/// the visual lightmap factor. Built-in timeline/weather layers are applied when registry data
/// does not provide the dimension timeline, matching `EnvironmentAttributeSystem` ordering.
pub(crate) fn entity_shadow_sky_darken(world: &WorldStore) -> u8 {
    let Some(level) = world.level_info() else {
        return 0;
    };
    let sampled_attributes = world
        .sampled_client_environment_attributes()
        .unwrap_or_else(|_| level.environment_attributes());
    let mut sky_light_level = sampled_attributes.sky_light_level.unwrap_or(15.0);
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    if vanilla_lightmap_dimension_kind(level) == VanillaLightmapDimensionKind::Overworld
        && uses_builtin_dimension_fallback
    {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        sky_light_level *= sample_periodic_float_keyframes(
            day_time,
            &VANILLA_OVERWORLD_SKY_LIGHT_LEVEL_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        );
    }
    if level.can_have_weather() {
        let weather = world.weather();
        let rain = sanitize_weather_lightmap_level(weather.rain_level);
        let thunder =
            (sanitize_weather_lightmap_level(weather.thunder_level) * rain).clamp(0.0, 1.0);
        let rain = (rain - thunder).max(0.0);
        sky_light_level =
            apply_weather_sky_light_level(sky_light_level, rain, VANILLA_WEATHER_RAIN_ALPHA);
        sky_light_level =
            apply_weather_sky_light_level(sky_light_level, thunder, VANILLA_WEATHER_THUNDER_ALPHA);
    }
    (15.0 - sky_light_level.clamp(0.0, 15.0))
        .trunc()
        .clamp(0.0, 15.0) as u8
}

fn apply_weather_sky_light_level(level: f32, weather_level: f32, alpha: f32) -> f32 {
    level + weather_level * alpha * (4.0 - level)
}

pub(super) fn sanitize_weather_lightmap_level(level: f32) -> f32 {
    if level.is_finite() {
        level.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn random_between_inclusive(
    random: &mut LevelEventSoundRandomState,
    min: i32,
    max_inclusive: i32,
) -> i32 {
    random.next_int_bound(max_inclusive - min + 1) + min
}

pub(super) fn dimension_lightmap_environment(
    level: &WorldLevelInfo,
    attributes: bbb_client_world::ClientEnvironmentAttributes,
) -> LightmapEnvironment {
    let mut environment = LightmapEnvironment::default();
    match vanilla_lightmap_dimension_kind(level) {
        VanillaLightmapDimensionKind::Nether => {
            environment.sky_factor = 0.0;
            environment.sky_light_color = VANILLA_NETHER_SKY_LIGHT_COLOR;
            environment.ambient_color = VANILLA_NETHER_AMBIENT_LIGHT_COLOR;
            environment.level_lighting = LevelLighting::Nether;
        }
        VanillaLightmapDimensionKind::End => {
            environment.sky_factor = 0.0;
            environment.sky_light_color = VANILLA_END_SKY_LIGHT_COLOR;
            environment.ambient_color = VANILLA_END_AMBIENT_LIGHT_COLOR;
        }
        VanillaLightmapDimensionKind::Overworld => {
            environment.sky_factor = VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR;
            environment.sky_light_color = VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR;
            environment.ambient_color = VANILLA_OVERWORLD_AMBIENT_LIGHT_COLOR;
        }
        VanillaLightmapDimensionKind::Other => {}
    }
    if let Some(value) = attributes.block_light_tint {
        environment.block_light_tint = rgb24(rgb_u8_to_argb(value));
    }
    if let Some(value) = attributes.sky_light_color {
        environment.sky_light_color = rgb24(rgb_u8_to_argb(value));
    }
    if let Some(value) = attributes.sky_light_factor {
        environment.sky_factor = value;
    }
    if let Some(value) = attributes.ambient_light_color {
        environment.ambient_color = rgb24(rgb_u8_to_argb(value));
    }
    if let Some(value) = attributes.night_vision_color {
        environment.night_vision_color = rgb24(rgb_u8_to_argb(value));
    }
    environment
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VanillaLightmapDimensionKind {
    Overworld,
    Nether,
    End,
    Other,
}

pub(super) fn vanilla_lightmap_dimension_kind(
    level: &WorldLevelInfo,
) -> VanillaLightmapDimensionKind {
    if let Some(dimension_type) = level.dimension_type_name.as_deref() {
        return match dimension_type {
            "minecraft:the_nether" => VanillaLightmapDimensionKind::Nether,
            "minecraft:the_end" => VanillaLightmapDimensionKind::End,
            "minecraft:overworld" | "minecraft:overworld_caves" => {
                VanillaLightmapDimensionKind::Overworld
            }
            _ => VanillaLightmapDimensionKind::Other,
        };
    }
    let dimension = level.dimension.as_str();
    match (level.dimension_type_id, dimension) {
        (1, _) | (_, "minecraft:the_nether") => VanillaLightmapDimensionKind::Nether,
        (2, _) | (_, "minecraft:the_end") => VanillaLightmapDimensionKind::End,
        (0, _) | (3, _) | (_, "minecraft:overworld") | (_, "minecraft:overworld_caves") => {
            VanillaLightmapDimensionKind::Overworld
        }
        _ => VanillaLightmapDimensionKind::Other,
    }
}

pub(super) fn local_player_effect(world: &WorldStore, effect_id: i32) -> Option<MobEffectState> {
    world
        .local_player_id()
        .and_then(|player_id| world.entity_effect(player_id, effect_id))
}

fn camera_entity_id(world: &WorldStore) -> Option<i32> {
    world.camera_entity_id()
}

pub(super) fn camera_entity_effect(world: &WorldStore, effect_id: i32) -> Option<MobEffectState> {
    camera_entity_id(world).and_then(|entity_id| world.entity_effect(entity_id, effect_id))
}

pub(super) fn camera_entity_is_living(world: &WorldStore) -> bool {
    camera_entity_id(world)
        .and_then(|entity_id| world.entity_type_id(entity_id))
        .is_some_and(vanilla_entity_type_is_living)
}

pub(super) fn camera_entity_is_spectator(world: &WorldStore) -> bool {
    let camera = world.local_player().camera;
    if camera.follows_player {
        return world.local_player_is_spectator();
    }
    let Some(entity_id) = camera.entity_id else {
        return false;
    };
    world.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
        && world
            .entity_uuid(entity_id)
            .and_then(|uuid| world.player_info_entry(uuid))
            .is_some_and(|info| info.is_spectator())
}

pub(super) fn camera_mob_effect_blocks_sky(world: &WorldStore) -> bool {
    camera_entity_effect(world, VANILLA_MOB_EFFECT_BLINDNESS_ID).is_some()
        || camera_entity_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID).is_some()
}

/// Vanilla `Gui.HeartType.forPlayer`: the base heart
/// sprite family, in precedence order poison > wither > fully-frozen > normal.
pub(super) fn camera_player_heart_type(player: HudCameraPlayerVitals) -> HudHeartKind {
    if player.poisoned {
        HudHeartKind::Poisoned
    } else if player.withered {
        HudHeartKind::Withered
    } else if player.fully_frozen {
        HudHeartKind::Frozen
    } else {
        HudHeartKind::Normal
    }
}
