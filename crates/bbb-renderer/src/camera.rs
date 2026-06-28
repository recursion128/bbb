use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

use crate::terrain;

/// Vanilla `Options.gamma` default. `LightmapRenderStateExtractor` forwards
/// this option as `LightmapRenderState.brightness` before darkness effects are
/// applied.
pub const VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR: f32 = 0.5;
/// Vanilla `LightmapRenderStateExtractor.extract`: `blockFactor` is
/// `blockLightFlicker + 1.4F`; before the first client tick the flicker term is
/// zero.
pub const VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR: f32 = 1.4;
/// Vanilla `EnvironmentAttributes.SKY_LIGHT_FACTOR` default.
pub const VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR: f32 = 1.0;
/// Vanilla `EnvironmentAttributes.BLOCK_LIGHT_TINT` default (`0xFFD88C`).
pub const VANILLA_DEFAULT_LIGHTMAP_BLOCK_LIGHT_TINT: [f32; 3] = [1.0, 216.0 / 255.0, 140.0 / 255.0];
/// Vanilla `EnvironmentAttributes.SKY_LIGHT_COLOR` default (`0xFFFFFF`).
pub const VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
/// Vanilla `EnvironmentAttributes.AMBIENT_LIGHT_COLOR` default (`0x000000`).
pub const VANILLA_DEFAULT_LIGHTMAP_AMBIENT_COLOR: [f32; 3] = [0.0, 0.0, 0.0];
/// Vanilla `EnvironmentAttributes.NIGHT_VISION_COLOR` default (`0x999999`).
pub const VANILLA_DEFAULT_LIGHTMAP_NIGHT_VISION_COLOR: [f32; 3] =
    [153.0 / 255.0, 153.0 / 255.0, 153.0 / 255.0];
/// Vanilla `Options.renderDistance` default.
pub const VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS: u32 = 12;
/// Vanilla `Options.renderDistance` lower bound.
pub const VANILLA_MIN_RENDER_DISTANCE_CHUNKS: u32 = 2;
/// Vanilla's high-memory `Options.renderDistance` upper bound. The official
/// client uses 16 on smaller heaps; bbb exposes the high-memory cap as an
/// explicit startup option.
pub const VANILLA_MAX_RENDER_DISTANCE_CHUNKS: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightmapEnvironment {
    pub sky_factor: f32,
    pub block_factor: f32,
    pub night_vision_factor: f32,
    pub darkness_scale: f32,
    pub boss_overlay_world_darkening: f32,
    pub brightness_factor: f32,
    pub block_light_tint: [f32; 3],
    pub sky_light_color: [f32; 3],
    pub ambient_color: [f32; 3],
    pub night_vision_color: [f32; 3],
}

impl Default for LightmapEnvironment {
    fn default() -> Self {
        Self {
            sky_factor: VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR,
            block_factor: VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR,
            night_vision_factor: 0.0,
            darkness_scale: 0.0,
            boss_overlay_world_darkening: 0.0,
            brightness_factor: VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
            block_light_tint: VANILLA_DEFAULT_LIGHTMAP_BLOCK_LIGHT_TINT,
            sky_light_color: VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
            ambient_color: VANILLA_DEFAULT_LIGHTMAP_AMBIENT_COLOR,
            night_vision_color: VANILLA_DEFAULT_LIGHTMAP_NIGHT_VISION_COLOR,
        }
    }
}

impl LightmapEnvironment {
    pub fn sanitized(self) -> Self {
        Self {
            sky_factor: sanitize_unit_factor(self.sky_factor, VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR),
            block_factor: sanitize_lightmap_block_factor(self.block_factor),
            night_vision_factor: sanitize_unit_factor(self.night_vision_factor, 0.0),
            darkness_scale: sanitize_unit_factor(self.darkness_scale, 0.0),
            boss_overlay_world_darkening: sanitize_unit_factor(
                self.boss_overlay_world_darkening,
                0.0,
            ),
            brightness_factor: sanitize_lightmap_brightness_factor(self.brightness_factor),
            block_light_tint: sanitize_rgb01(
                self.block_light_tint,
                VANILLA_DEFAULT_LIGHTMAP_BLOCK_LIGHT_TINT,
            ),
            sky_light_color: sanitize_rgb01(
                self.sky_light_color,
                VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
            ),
            ambient_color: sanitize_rgb01(
                self.ambient_color,
                VANILLA_DEFAULT_LIGHTMAP_AMBIENT_COLOR,
            ),
            night_vision_color: sanitize_rgb01(
                self.night_vision_color,
                VANILLA_DEFAULT_LIGHTMAP_NIGHT_VISION_COLOR,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogEnvironment {
    pub color: [f32; 4],
    pub environmental_start: f32,
    pub environmental_end: f32,
    pub render_distance_start: f32,
    pub render_distance_end: f32,
    pub sky_end: f32,
    pub cloud_end: f32,
}

impl Default for FogEnvironment {
    fn default() -> Self {
        Self::disabled()
    }
}

impl FogEnvironment {
    pub fn disabled() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 0.0],
            environmental_start: 0.0,
            environmental_end: 0.0,
            render_distance_start: 0.0,
            render_distance_end: 0.0,
            sky_end: 0.0,
            cloud_end: 0.0,
        }
    }

    pub fn world(
        color: [f32; 4],
        environmental_start: f32,
        environmental_end: f32,
        render_distance_chunks: u32,
    ) -> Self {
        Self::world_with_visibility_ends(
            color,
            environmental_start,
            environmental_end,
            render_distance_chunks,
            environmental_end,
            environmental_end,
        )
    }

    pub fn world_with_visibility_ends(
        color: [f32; 4],
        environmental_start: f32,
        environmental_end: f32,
        render_distance_chunks: u32,
        sky_end: f32,
        cloud_end: f32,
    ) -> Self {
        let (render_distance_start, render_distance_end) =
            vanilla_render_distance_fog_range(render_distance_chunks);
        Self {
            color,
            environmental_start,
            environmental_end,
            render_distance_start,
            render_distance_end,
            sky_end,
            cloud_end,
        }
        .sanitized()
    }

    pub fn sanitized(self) -> Self {
        Self {
            color: [
                sanitize_unit_factor(self.color[0], 0.0),
                sanitize_unit_factor(self.color[1], 0.0),
                sanitize_unit_factor(self.color[2], 0.0),
                sanitize_unit_factor(self.color[3], 0.0),
            ],
            environmental_start: sanitize_fog_distance(self.environmental_start, 0.0),
            environmental_end: sanitize_fog_distance(self.environmental_end, 0.0),
            render_distance_start: sanitize_fog_distance(self.render_distance_start, 0.0),
            render_distance_end: sanitize_fog_distance(self.render_distance_end, 0.0),
            sky_end: sanitize_fog_distance(self.sky_end, 0.0),
            cloud_end: sanitize_fog_distance(self.cloud_end, 0.0),
        }
    }
}

pub fn vanilla_render_distance_fog_range(render_distance_chunks: u32) -> (f32, f32) {
    let chunks = render_distance_chunks.clamp(
        VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
    ) as f32;
    let render_distance_blocks = chunks * 16.0;
    let span = (render_distance_blocks / 10.0).clamp(4.0, 64.0);
    (render_distance_blocks - span, render_distance_blocks)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClearColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for ClearColor {
    fn default() -> Self {
        Self {
            r: 0.04,
            g: 0.07,
            b: 0.10,
            a: 1.0,
        }
    }
}

impl From<ClearColor> for wgpu::Color {
    fn from(value: ClearColor) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraPose {
    pub position: [f32; 3],
    pub y_rot: f32,
    pub x_rot: f32,
    pub eye_height: f32,
}

impl CameraPose {
    pub const STANDING_EYE_HEIGHT: f32 = 1.62;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    /// Vanilla `LightmapInfo` scalar factors:
    /// `[SkyFactor, BlockFactor, NightVisionFactor, DarknessScale]`.
    lightmap_factors: [f32; 4],
    /// Vanilla `LightmapInfo` effect factors:
    /// `[BossOverlayWorldDarkeningFactor, BrightnessFactor, _, _]`.
    lightmap_effects: [f32; 4],
    block_light_tint: [f32; 4],
    sky_light_color: [f32; 4],
    ambient_color: [f32; 4],
    night_vision_color: [f32; 4],
    camera_position: [f32; 4],
    fog_color: [f32; 4],
    /// Vanilla fog distances:
    /// `[EnvironmentalStart, EnvironmentalEnd, RenderDistanceStart, RenderDistanceEnd]`.
    fog_distances: [f32; 4],
    /// Vanilla `FogData` sky/cloud end distances:
    /// `[SkyEnd, CloudEnd, _, _]`.
    fog_visibility_ends: [f32; 4],
}

impl CameraUniform {
    pub(crate) fn identity() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            ..Self::from_lightmap_environment(LightmapEnvironment::default())
        }
    }

    /// The GUI orthographic projection for 3D inventory item icons (vanilla `Projection.setupOrtho`,
    /// `invertY`): `setOrtho(0, width, height, 0, -1000, 1000)` over a `0..1` (wgpu) depth range. Maps GUI
    /// pixel space (origin top-left, y down) to clip space, so a block item baked at its slot pixel rect
    /// projects into the slot. `width`/`height` are the surface size in pixels.
    pub(crate) fn gui_ortho(width: f32, height: f32) -> Self {
        let projection =
            Mat4::orthographic_rh(0.0, width.max(1.0), height.max(1.0), 0.0, -1000.0, 1000.0);
        Self {
            view_proj: projection.to_cols_array_2d(),
            ..Self::identity()
        }
    }

    pub(crate) fn from_bounds(bounds: TerrainBounds, aspect: f32) -> Self {
        let center = bounds.center();
        let extent = bounds.extent();
        let radius = extent.length().max(48.0);
        let eye = center + Vec3::new(radius * 0.55, radius * 0.42, radius * 0.78);
        let view = Mat4::look_at_rh(eye, center, Vec3::Y);

        let half_height = (extent.y * 0.65 + extent.x.max(extent.z) * 0.45).max(40.0);
        let half_width = half_height * aspect.max(0.1);
        let far = radius * 5.0 + 512.0;
        let projection =
            Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, 0.1, far);

        Self {
            view_proj: (projection * view).to_cols_array_2d(),
            camera_position: vec3_to_vec4(eye),
            ..Self::identity()
        }
    }

    pub(crate) fn from_pose(pose: CameraPose, aspect: f32) -> Self {
        let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
        let yaw = pose.y_rot.to_radians();
        let pitch = pose.x_rot.to_radians();
        let cos_pitch = pitch.cos();
        let forward = Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch)
            .normalize_or_zero();
        let target = eye
            + if forward.length_squared() > 0.0 {
                forward
            } else {
                Vec3::Z
            };
        let projection = Mat4::perspective_rh(70.0_f32.to_radians(), aspect.max(0.1), 0.05, 2048.0);
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);

        Self {
            view_proj: (projection * view).to_cols_array_2d(),
            camera_position: vec3_to_vec4(eye),
            ..Self::identity()
        }
    }

    #[cfg(test)]
    pub(crate) fn with_lightmap_brightness_factor(mut self, factor: f32) -> Self {
        self.lightmap_effects[1] = sanitize_lightmap_brightness_factor(factor);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_lightmap_block_factor(mut self, factor: f32) -> Self {
        self.lightmap_factors[1] = sanitize_lightmap_block_factor(factor);
        self
    }

    pub(crate) fn with_lightmap_environment(mut self, environment: LightmapEnvironment) -> Self {
        let lightmap = Self::from_lightmap_environment(environment);
        self.lightmap_factors = lightmap.lightmap_factors;
        self.lightmap_effects = lightmap.lightmap_effects;
        self.block_light_tint = lightmap.block_light_tint;
        self.sky_light_color = lightmap.sky_light_color;
        self.ambient_color = lightmap.ambient_color;
        self.night_vision_color = lightmap.night_vision_color;
        self
    }

    pub(crate) fn with_fog_environment(mut self, environment: FogEnvironment) -> Self {
        let environment = environment.sanitized();
        self.fog_color = environment.color;
        self.fog_distances = [
            environment.environmental_start,
            environment.environmental_end,
            environment.render_distance_start,
            environment.render_distance_end,
        ];
        self.fog_visibility_ends = [environment.sky_end, environment.cloud_end, 0.0, 0.0];
        self
    }

    #[cfg(test)]
    pub(crate) fn lightmap_brightness_factor(self) -> f32 {
        self.lightmap_effects[1]
    }

    #[cfg(test)]
    pub(crate) fn lightmap_block_factor(self) -> f32 {
        self.lightmap_factors[1]
    }

    #[cfg(test)]
    pub(crate) fn lightmap_environment(self) -> LightmapEnvironment {
        LightmapEnvironment {
            sky_factor: self.lightmap_factors[0],
            block_factor: self.lightmap_factors[1],
            night_vision_factor: self.lightmap_factors[2],
            darkness_scale: self.lightmap_factors[3],
            boss_overlay_world_darkening: self.lightmap_effects[0],
            brightness_factor: self.lightmap_effects[1],
            block_light_tint: self.block_light_tint[0..3].try_into().unwrap(),
            sky_light_color: self.sky_light_color[0..3].try_into().unwrap(),
            ambient_color: self.ambient_color[0..3].try_into().unwrap(),
            night_vision_color: self.night_vision_color[0..3].try_into().unwrap(),
        }
    }

    #[cfg(test)]
    pub(crate) fn camera_position(self) -> [f32; 3] {
        self.camera_position[0..3].try_into().unwrap()
    }

    #[cfg(test)]
    pub(crate) fn fog_environment(self) -> FogEnvironment {
        FogEnvironment {
            color: self.fog_color,
            environmental_start: self.fog_distances[0],
            environmental_end: self.fog_distances[1],
            render_distance_start: self.fog_distances[2],
            render_distance_end: self.fog_distances[3],
            sky_end: self.fog_visibility_ends[0],
            cloud_end: self.fog_visibility_ends[1],
        }
    }

    fn from_lightmap_environment(environment: LightmapEnvironment) -> Self {
        let environment = environment.sanitized();
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            lightmap_factors: [
                environment.sky_factor,
                environment.block_factor,
                environment.night_vision_factor,
                environment.darkness_scale,
            ],
            lightmap_effects: [
                environment.boss_overlay_world_darkening,
                environment.brightness_factor,
                0.0,
                0.0,
            ],
            block_light_tint: rgb_to_vec4(environment.block_light_tint),
            sky_light_color: rgb_to_vec4(environment.sky_light_color),
            ambient_color: rgb_to_vec4(environment.ambient_color),
            night_vision_color: rgb_to_vec4(environment.night_vision_color),
            camera_position: [0.0, 0.0, 0.0, 0.0],
            fog_color: FogEnvironment::disabled().color,
            fog_distances: [
                FogEnvironment::disabled().environmental_start,
                FogEnvironment::disabled().environmental_end,
                FogEnvironment::disabled().render_distance_start,
                FogEnvironment::disabled().render_distance_end,
            ],
            fog_visibility_ends: [
                FogEnvironment::disabled().sky_end,
                FogEnvironment::disabled().cloud_end,
                0.0,
                0.0,
            ],
        }
    }
}

pub(crate) fn sanitize_lightmap_brightness_factor(factor: f32) -> f32 {
    if factor.is_finite() {
        factor.clamp(0.0, 1.0)
    } else {
        VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
    }
}

pub(crate) fn sanitize_lightmap_block_factor(factor: f32) -> f32 {
    if factor.is_finite() {
        factor.max(0.0)
    } else {
        VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR
    }
}

fn sanitize_unit_factor(factor: f32, fallback: f32) -> f32 {
    if factor.is_finite() {
        factor.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn sanitize_rgb01(color: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    [
        sanitize_unit_factor(color[0], fallback[0]),
        sanitize_unit_factor(color[1], fallback[1]),
        sanitize_unit_factor(color[2], fallback[2]),
    ]
}

fn sanitize_fog_distance(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn rgb_to_vec4(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 0.0]
}

fn vec3_to_vec4(value: Vec3) -> [f32; 4] {
    [value.x, value.y, value.z, 0.0]
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainBounds {
    min: Vec3,
    max: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_uniform_defaults_to_vanilla_lightmap_brightness_factor() {
        assert_eq!(
            CameraUniform::identity().lightmap_brightness_factor(),
            VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
        );
        assert_eq!(
            CameraUniform::identity().lightmap_block_factor(),
            VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR
        );
        assert_eq!(
            CameraUniform::gui_ortho(320.0, 240.0).lightmap_brightness_factor(),
            VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
        );
    }

    #[test]
    fn camera_uniform_defaults_to_vanilla_lightmap_environment() {
        let environment = CameraUniform::identity().lightmap_environment();
        assert_eq!(environment.sky_factor, VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR);
        assert_eq!(
            environment.block_factor,
            VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR
        );
        assert_eq!(environment.night_vision_factor, 0.0);
        assert_eq!(environment.darkness_scale, 0.0);
        assert_eq!(environment.boss_overlay_world_darkening, 0.0);
        assert_eq!(
            environment.brightness_factor,
            VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
        );
        assert_eq!(
            environment.block_light_tint,
            VANILLA_DEFAULT_LIGHTMAP_BLOCK_LIGHT_TINT
        );
        assert_eq!(
            environment.sky_light_color,
            VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR
        );
        assert_eq!(
            environment.ambient_color,
            VANILLA_DEFAULT_LIGHTMAP_AMBIENT_COLOR
        );
        assert_eq!(
            environment.night_vision_color,
            VANILLA_DEFAULT_LIGHTMAP_NIGHT_VISION_COLOR
        );
    }

    #[test]
    fn camera_uniform_defaults_to_disabled_fog_environment() {
        assert_eq!(
            CameraUniform::identity().fog_environment(),
            FogEnvironment::disabled()
        );
        assert_eq!(
            CameraUniform::gui_ortho(320.0, 240.0).fog_environment(),
            FogEnvironment::disabled()
        );
    }

    #[test]
    fn camera_uniform_stores_camera_position_for_world_projections() {
        let pose = CameraPose {
            position: [10.0, 64.0, -5.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };

        assert_eq!(
            CameraUniform::from_pose(pose, 16.0 / 9.0).camera_position(),
            [10.0, 65.62, -5.0]
        );
    }

    #[test]
    fn camera_uniform_stores_fog_environment() {
        let fog = FogEnvironment::world_with_visibility_ends(
            [0.25, 0.5, 0.75, 1.0],
            -8.0,
            96.0,
            12,
            192.0,
            2048.0,
        );

        assert_eq!(
            CameraUniform::identity()
                .with_fog_environment(fog)
                .fog_environment(),
            fog
        );
    }

    #[test]
    fn vanilla_render_distance_fog_range_matches_fog_renderer_span() {
        assert_eq!(vanilla_render_distance_fog_range(2), (28.0, 32.0));
        assert_eq!(vanilla_render_distance_fog_range(12), (172.8, 192.0));
        assert_eq!(vanilla_render_distance_fog_range(32), (460.8, 512.0));
    }

    #[test]
    fn camera_uniform_clamps_lightmap_brightness_factor() {
        assert_eq!(
            CameraUniform::identity()
                .with_lightmap_brightness_factor(-1.0)
                .lightmap_brightness_factor(),
            0.0
        );
        assert_eq!(
            CameraUniform::identity()
                .with_lightmap_brightness_factor(2.0)
                .lightmap_brightness_factor(),
            1.0
        );
        assert_eq!(
            CameraUniform::identity()
                .with_lightmap_brightness_factor(f32::NAN)
                .lightmap_brightness_factor(),
            VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
        );
    }

    #[test]
    fn camera_uniform_clamps_lightmap_block_factor() {
        assert_eq!(
            CameraUniform::identity()
                .with_lightmap_block_factor(-1.0)
                .lightmap_block_factor(),
            0.0
        );
        assert_eq!(
            CameraUniform::identity()
                .with_lightmap_block_factor(1.25)
                .lightmap_block_factor(),
            1.25
        );
        assert_eq!(
            CameraUniform::identity()
                .with_lightmap_block_factor(f32::NAN)
                .lightmap_block_factor(),
            VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR
        );
    }

    #[test]
    fn camera_uniform_sanitizes_lightmap_environment() {
        let environment = LightmapEnvironment {
            sky_factor: 1.5,
            block_factor: -2.0,
            night_vision_factor: f32::NAN,
            darkness_scale: 0.25,
            boss_overlay_world_darkening: -0.5,
            brightness_factor: f32::INFINITY,
            block_light_tint: [1.2, -0.2, f32::NAN],
            sky_light_color: [0.25, 2.0, 0.75],
            ambient_color: [f32::NAN, 0.5, -1.0],
            night_vision_color: [0.2, f32::INFINITY, 1.5],
        };
        let sanitized = CameraUniform::identity()
            .with_lightmap_environment(environment)
            .lightmap_environment();

        assert_eq!(sanitized.sky_factor, 1.0);
        assert_eq!(sanitized.block_factor, 0.0);
        assert_eq!(sanitized.night_vision_factor, 0.0);
        assert_eq!(sanitized.darkness_scale, 0.25);
        assert_eq!(sanitized.boss_overlay_world_darkening, 0.0);
        assert_eq!(
            sanitized.brightness_factor,
            VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
        );
        assert_eq!(
            sanitized.block_light_tint,
            [1.0, 0.0, VANILLA_DEFAULT_LIGHTMAP_BLOCK_LIGHT_TINT[2]]
        );
        assert_eq!(sanitized.sky_light_color, [0.25, 1.0, 0.75]);
        assert_eq!(
            sanitized.ambient_color,
            [VANILLA_DEFAULT_LIGHTMAP_AMBIENT_COLOR[0], 0.5, 0.0]
        );
        assert_eq!(
            sanitized.night_vision_color,
            [0.2, VANILLA_DEFAULT_LIGHTMAP_NIGHT_VISION_COLOR[1], 1.0]
        );
    }

    #[test]
    fn camera_uniform_sanitizes_fog_environment() {
        let fog = FogEnvironment {
            color: [1.25, -1.0, f32::NAN, 2.0],
            environmental_start: f32::NAN,
            environmental_end: f32::INFINITY,
            render_distance_start: -16.0,
            render_distance_end: 128.0,
            sky_end: f32::NAN,
            cloud_end: 2048.0,
        };

        assert_eq!(
            CameraUniform::identity()
                .with_fog_environment(fog)
                .fog_environment(),
            FogEnvironment {
                color: [1.0, 0.0, 0.0, 1.0],
                environmental_start: 0.0,
                environmental_end: 0.0,
                render_distance_start: -16.0,
                render_distance_end: 128.0,
                sky_end: 0.0,
                cloud_end: 2048.0,
            }
        );
    }
}

impl TerrainBounds {
    pub(crate) fn from_vertices(vertices: &[terrain::TerrainVertex]) -> Option<Self> {
        Self::from_points(
            vertices
                .iter()
                .map(|vertex| Vec3::from_array(vertex.position)),
        )
    }

    pub(crate) fn from_points(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let mut bounds = Self {
            min: first,
            max: first,
        };
        for point in points {
            bounds.include_point(point);
        }
        Some(bounds)
    }

    fn include_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    pub(crate) fn include_bounds(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn extent(self) -> Vec3 {
        (self.max - self.min).max(Vec3::splat(1.0))
    }
}
