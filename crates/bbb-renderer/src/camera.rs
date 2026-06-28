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

fn rgb_to_vec4(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 0.0]
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
