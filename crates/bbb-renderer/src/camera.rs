use glam::{EulerRot, Mat4, Vec3, Vec4};
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
/// Vanilla `Options.glintSpeed` default.
pub(crate) const VANILLA_DEFAULT_GLINT_SPEED: f64 = 0.5;
/// Vanilla `ProjectionType.PERSPECTIVE.applyLayeringTransform(..., 1.0F)`.
pub(crate) const VANILLA_VIEW_OFFSET_Z_PERSPECTIVE_SCALE: f32 = 1.0 - 1.0 / 4096.0;
/// Vanilla `ProjectionType.ORTHOGRAPHIC.applyLayeringTransform(..., 1.0F)`.
pub(crate) const VANILLA_VIEW_OFFSET_Z_ORTHOGRAPHIC_TRANSLATE: f32 = 1.0 / 512.0;
/// Vanilla `ProjectionType.PERSPECTIVE.applyLayeringTransform(..., -1.0F)`.
pub(crate) const VANILLA_VIEW_OFFSET_Z_FORWARD_PERSPECTIVE_SCALE: f32 = 1.0 + 1.0 / 4096.0;
/// Vanilla `ProjectionType.ORTHOGRAPHIC.applyLayeringTransform(..., -1.0F)`.
pub(crate) const VANILLA_VIEW_OFFSET_Z_FORWARD_ORTHOGRAPHIC_TRANSLATE: f32 = -1.0 / 512.0;
/// Vanilla `Options.renderDistance` default.
pub const VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS: u32 = 12;
/// Vanilla `Options.renderDistance` lower bound.
pub const VANILLA_MIN_RENDER_DISTANCE_CHUNKS: u32 = 2;
/// Vanilla's high-memory `Options.renderDistance` upper bound. The official
/// client uses 16 on smaller heaps; bbb exposes the high-memory cap as an
/// explicit startup option.
pub const VANILLA_MAX_RENDER_DISTANCE_CHUNKS: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelLighting {
    Default,
    Nether,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LightingEntry {
    LevelDefault,
    LevelNether,
    #[allow(dead_code)]
    ItemsFlat,
    Items3d,
    EntityInUi,
}

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
    pub level_lighting: LevelLighting,
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
            level_lighting: LevelLighting::Default,
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
            level_lighting: self.level_lighting,
        }
    }
}

impl LevelLighting {
    fn entry(self) -> LightingEntry {
        match self {
            LevelLighting::Default => LightingEntry::LevelDefault,
            LevelLighting::Nether => LightingEntry::LevelNether,
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

/// Vanilla `GameRenderer.bobHurt` inputs for the active living camera entity.
/// The transform is presentation-only and is applied in projection space, so it
/// does not alter the canonical camera pose used for picking or interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraHurtEffect {
    pub hurt_time: f32,
    pub hurt_duration: f32,
    pub hurt_dir: f32,
    pub death_time: f32,
    pub is_dead_or_dying: bool,
    pub damage_tilt_strength: f32,
}

/// Vanilla `GameRenderer.bobView` inputs copied from the active player
/// camera's `ClientAvatarState`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraViewBobEffect {
    pub backwards_interpolated_walk_distance: f32,
    pub bob: f32,
}

/// Vanilla `GameRenderer`'s portal/nausea projection distortion after the
/// camera hurt/view-bob transform. Native currently supplies the nausea
/// contribution; keeping the renderer input generic preserves the shared
/// projection owner for a future portal contribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSpinningEffect {
    pub intensity: f32,
    pub angle_degrees: f32,
}

impl CameraPose {
    pub const STANDING_EYE_HEIGHT: f32 = 1.62;

    pub(crate) fn billboard_axes(self) -> CameraBillboardAxes {
        let yaw = self.y_rot.to_radians();
        let pitch = self.x_rot.to_radians();
        let cos_pitch = pitch.cos();
        let forward = Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch)
            .normalize_or_zero();
        let forward = if forward.length_squared() > 0.0 {
            forward
        } else {
            Vec3::Z
        };
        // Camera.setRotation uses `rotationYXZ(PI-yaw, -pitch, 0)`; local +X
        // therefore maps to screen-right (`forward x world-up`), which is -X
        // for vanilla yaw 0 (looking south). The older `Y x forward` formula
        // produced screen-left and back-facing culled quads.
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let right = if right.length_squared() > 0.0 {
            right
        } else {
            Vec3::X
        };
        let up = right.cross(forward).normalize_or_zero();
        CameraBillboardAxes {
            forward,
            right,
            up: if up.length_squared() > 0.0 {
                up
            } else {
                Vec3::Y
            },
        }
    }
}

/// Immutable perspective frustum used by render extraction before building
/// heap-heavy entity render state. It shares the renderer's camera convention
/// and WGPU zero-to-one clip depth.
#[derive(Debug, Clone, Copy)]
pub struct CameraFrustum {
    view_projection: Mat4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AabbFrustumIntersection {
    Outside,
    Intersecting,
    Inside,
}

impl CameraFrustum {
    pub fn from_pose(pose: CameraPose, aspect: f32, fov_degrees: f32) -> Self {
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
        let fov_degrees = sanitize_perspective_fov_degrees(fov_degrees);
        let projection =
            Mat4::perspective_rh(fov_degrees.to_radians(), aspect.max(0.1), 0.05, 2048.0);
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);
        Self {
            view_projection: projection * view,
        }
    }

    /// Conservative AABB/frustum intersection. Invalid coordinates remain
    /// visible so malformed presentation data cannot hide a canonical entity.
    pub fn intersects_aabb(self, min: [f64; 3], max: [f64; 3]) -> bool {
        self.classify_aabb(min, max) != AabbFrustumIntersection::Outside
    }

    /// Classifies an AABB for hierarchical traversal. Invalid coordinates are
    /// intersecting so callers descend conservatively rather than hiding data.
    pub fn classify_aabb(self, min: [f64; 3], max: [f64; 3]) -> AabbFrustumIntersection {
        if min
            .into_iter()
            .chain(max)
            .any(|coordinate| !coordinate.is_finite())
        {
            return AabbFrustumIntersection::Intersecting;
        }
        let corners = [
            [min[0], min[1], min[2]],
            [min[0], min[1], max[2]],
            [min[0], max[1], min[2]],
            [min[0], max[1], max[2]],
            [max[0], min[1], min[2]],
            [max[0], min[1], max[2]],
            [max[0], max[1], min[2]],
            [max[0], max[1], max[2]],
        ]
        .map(|corner| {
            self.view_projection
                * Vec4::new(corner[0] as f32, corner[1] as f32, corner[2] as f32, 1.0)
        });
        let outside_left = corners.iter().all(|corner| corner.x < -corner.w);
        let outside_right = corners.iter().all(|corner| corner.x > corner.w);
        let outside_bottom = corners.iter().all(|corner| corner.y < -corner.w);
        let outside_top = corners.iter().all(|corner| corner.y > corner.w);
        let outside_near = corners.iter().all(|corner| corner.z < 0.0);
        let outside_far = corners.iter().all(|corner| corner.z > corner.w);
        if outside_left
            || outside_right
            || outside_bottom
            || outside_top
            || outside_near
            || outside_far
        {
            return AabbFrustumIntersection::Outside;
        }
        let fully_inside = corners.iter().all(|corner| {
            corner.x >= -corner.w
                && corner.x <= corner.w
                && corner.y >= -corner.w
                && corner.y <= corner.w
                && corner.z >= 0.0
                && corner.z <= corner.w
        });
        if fully_inside {
            AabbFrustumIntersection::Inside
        } else {
            AabbFrustumIntersection::Intersecting
        }
    }
}

/// Camera-facing basis shared by item, particle, and debug-text billboards.
/// `forward` points from the camera into the scene; `right`/`up` are the
/// billboard plane axes used after vanilla's camera-orientation transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CameraBillboardAxes {
    pub(crate) forward: Vec3,
    pub(crate) right: Vec3,
    pub(crate) up: Vec3,
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
    /// Vanilla `Lighting.Entry` diffuse directions used by lit model shaders.
    minecraft_light0: [f32; 4],
    minecraft_light1: [f32; 4],
    /// Vanilla `TextureTransform.setupGlintTexturing` translation offsets:
    /// `[-layerOffset0, layerOffset1, _, _]`.
    glint_offsets: [f32; 4],
    /// Vanilla `LayeringTransform.VIEW_OFFSET_Z_LAYERING` view-projection matrix.
    view_proj_view_offset_z: [[f32; 4]; 4],
    /// Vanilla `LayeringTransform.VIEW_OFFSET_Z_LAYERING_FORWARD` view-projection matrix.
    view_proj_view_offset_z_forward: [[f32; 4]; 4],
    /// Vanilla projection matrix for sky shaders that bind `DynamicTransforms.ModelViewMat`
    /// separately from `Projection.ProjMat`.
    projection: [[f32; 4]; 4],
    /// Vanilla sky model-view matrix without camera translation (`CameraRenderState.viewRotationMatrix`).
    sky_model_view: [[f32; 4]; 4],
    /// Vanilla `GlobalSettingsUniform.GameTime` in shader units:
    /// `((gameTime % 24000) + partialTick) / 24000`.
    shader_game_time: [f32; 4],
    /// Vanilla line shaders consume `ScreenSize` for pixel-width line expansion.
    /// `[width, height, _, _]`.
    viewport_size: [f32; 4],
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
            projection: projection.to_cols_array_2d(),
            sky_model_view: Mat4::IDENTITY.to_cols_array_2d(),
            view_proj_view_offset_z: orthographic_view_offset_z_projection(
                projection,
                Mat4::IDENTITY,
            )
            .to_cols_array_2d(),
            view_proj_view_offset_z_forward: orthographic_view_offset_z_forward_projection(
                projection,
                Mat4::IDENTITY,
            )
            .to_cols_array_2d(),
            ..Self::identity().with_lighting_entry(LightingEntry::Items3d)
        }
    }

    /// The camera for one GUI entity picture-in-picture target (vanilla
    /// `PictureInPictureRenderer.prepare` + `GuiEntityRenderer`): the private texture's ortho
    /// projection `setupOrtho(-1000, 1000, width, height, invertY)` (the same y-down GUI pixel ortho
    /// as [`Self::gui_ortho`]) with the pose-stack chain folded into the view-projection —
    /// `translate(width/2, height/2, 0)` (`GuiEntityRenderer.getTranslateY` returns `height / 2`)
    /// `· scale(s, s, -s) · translate(state.translation) · rotate(state.rotation)` — under
    /// `Lighting.Entry.ENTITY_IN_UI`. bbb draws HUD GUI pixels 1:1 with surface pixels, so the
    /// vanilla `guiScale` factor in both the texture size and the pose scale is 1 and
    /// `s == state.scale`. `width`/`height` are the PIP texture size in pixels; `rotation` is a
    /// `[x, y, z, w]` quaternion (JOML `Quaternionf` order). Vanilla's blit-side vertical UV flip is
    /// a GL framebuffer-origin artifact; under wgpu (row 0 = top) the rendered texture is already
    /// GUI-oriented and the blit samples it with identity UVs.
    pub(crate) fn hud_entity_preview_pip(
        width: f32,
        height: f32,
        scale: f32,
        translation: [f32; 3],
        rotation: [f32; 4],
    ) -> Self {
        Self::hud_picture_in_picture(
            width,
            height,
            scale,
            translation,
            rotation,
            LightingEntry::EntityInUi,
        )
    }

    pub(crate) fn hud_picture_in_picture(
        width: f32,
        height: f32,
        scale: f32,
        translation: [f32; 3],
        rotation: [f32; 4],
        lighting: LightingEntry,
    ) -> Self {
        let width = width.max(1.0);
        let height = height.max(1.0);
        let projection = Mat4::orthographic_rh(0.0, width, height, 0.0, -1000.0, 1000.0);
        let rotation = glam::Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]);
        let rotation = if rotation.length_squared() > f32::EPSILON {
            rotation.normalize()
        } else {
            glam::Quat::IDENTITY
        };
        let model = Mat4::from_translation(Vec3::new(width / 2.0, height / 2.0, 0.0))
            * Mat4::from_scale(Vec3::new(scale, scale, -scale))
            * Mat4::from_translation(Vec3::from_array(translation))
            * Mat4::from_quat(rotation);
        Self {
            view_proj: (projection * model).to_cols_array_2d(),
            projection: projection.to_cols_array_2d(),
            sky_model_view: Mat4::IDENTITY.to_cols_array_2d(),
            view_proj_view_offset_z: orthographic_view_offset_z_projection(projection, model)
                .to_cols_array_2d(),
            view_proj_view_offset_z_forward: orthographic_view_offset_z_forward_projection(
                projection, model,
            )
            .to_cols_array_2d(),
            ..Self::identity().with_lighting_entry(lighting)
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
            projection: projection.to_cols_array_2d(),
            sky_model_view: view_without_translation(view).to_cols_array_2d(),
            view_proj_view_offset_z: orthographic_view_offset_z_projection(projection, view)
                .to_cols_array_2d(),
            view_proj_view_offset_z_forward: orthographic_view_offset_z_forward_projection(
                projection, view,
            )
            .to_cols_array_2d(),
            camera_position: vec3_to_vec4(eye),
            ..Self::identity()
        }
    }

    #[cfg(test)]
    pub(crate) fn from_pose(pose: CameraPose, aspect: f32) -> Self {
        Self::from_pose_with_fov(pose, aspect, 70.0)
    }

    #[cfg(test)]
    pub(crate) fn from_pose_with_fov(pose: CameraPose, aspect: f32, fov_degrees: f32) -> Self {
        Self::from_pose_with_fov_and_hurt(pose, aspect, fov_degrees, None)
    }

    #[cfg(test)]
    pub(crate) fn from_pose_with_fov_and_hurt(
        pose: CameraPose,
        aspect: f32,
        fov_degrees: f32,
        hurt: Option<CameraHurtEffect>,
    ) -> Self {
        Self::from_pose_with_effects(pose, aspect, fov_degrees, hurt, None)
    }

    #[cfg(test)]
    pub(crate) fn from_pose_with_effects(
        pose: CameraPose,
        aspect: f32,
        fov_degrees: f32,
        hurt: Option<CameraHurtEffect>,
        spinning: Option<CameraSpinningEffect>,
    ) -> Self {
        Self::from_pose_with_camera_effects(pose, aspect, fov_degrees, hurt, None, spinning)
    }

    pub(crate) fn from_pose_with_camera_effects(
        pose: CameraPose,
        aspect: f32,
        fov_degrees: f32,
        hurt: Option<CameraHurtEffect>,
        view_bob: Option<CameraViewBobEffect>,
        spinning: Option<CameraSpinningEffect>,
    ) -> Self {
        Self::from_pose_with_effects_and_far(
            pose,
            aspect,
            fov_degrees,
            hurt,
            view_bob,
            spinning,
            2048.0,
        )
    }

    #[cfg(test)]
    pub(crate) fn from_hud_pose(
        pose: CameraPose,
        aspect: f32,
        fov_degrees: f32,
        hurt: Option<CameraHurtEffect>,
    ) -> Self {
        Self::from_hud_pose_with_view_bob(pose, aspect, fov_degrees, hurt, None)
    }

    pub(crate) fn from_hud_pose_with_view_bob(
        pose: CameraPose,
        aspect: f32,
        fov_degrees: f32,
        hurt: Option<CameraHurtEffect>,
        view_bob: Option<CameraViewBobEffect>,
    ) -> Self {
        Self::from_pose_with_effects_and_far(pose, aspect, fov_degrees, hurt, view_bob, None, 100.0)
    }

    fn from_pose_with_effects_and_far(
        pose: CameraPose,
        aspect: f32,
        fov_degrees: f32,
        hurt: Option<CameraHurtEffect>,
        view_bob: Option<CameraViewBobEffect>,
        spinning: Option<CameraSpinningEffect>,
        z_far: f32,
    ) -> Self {
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
        let fov_degrees = sanitize_perspective_fov_degrees(fov_degrees);
        let projection =
            Mat4::perspective_rh(fov_degrees.to_radians(), aspect.max(0.1), 0.05, z_far);
        // Vanilla `GameRenderer.renderLevel`: `projectionMatrix.mul(bobStack.last().pose())`.
        // `bobHurt` is therefore camera-space presentation state on the projection side of
        // the view matrix, not part of `CameraRenderState.viewRotationMatrix`.
        let projection = projection
            * camera_hurt_transform(hurt)
            * camera_view_bob_transform(view_bob)
            * camera_spinning_transform(spinning);
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);

        Self {
            view_proj: (projection * view).to_cols_array_2d(),
            projection: projection.to_cols_array_2d(),
            sky_model_view: view_without_translation(view).to_cols_array_2d(),
            view_proj_view_offset_z: perspective_view_offset_z_projection(projection, view)
                .to_cols_array_2d(),
            view_proj_view_offset_z_forward: perspective_view_offset_z_forward_projection(
                projection, view,
            )
            .to_cols_array_2d(),
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
        self.minecraft_light0 = lightmap.minecraft_light0;
        self.minecraft_light1 = lightmap.minecraft_light1;
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

    pub(crate) fn with_lighting_entry(mut self, entry: LightingEntry) -> Self {
        let (light0, light1) = lighting_entry_directions(entry);
        self.minecraft_light0 = vec3_to_vec4(light0);
        self.minecraft_light1 = vec3_to_vec4(light1);
        self
    }

    pub(crate) fn with_glint_texture_time(mut self, elapsed_millis: f64, glint_speed: f64) -> Self {
        let offsets = vanilla_glint_texture_offsets(elapsed_millis, glint_speed);
        self.glint_offsets = [offsets[0], offsets[1], 0.0, 0.0];
        self
    }

    pub(crate) fn with_shader_game_time_ticks(mut self, ticks: f64) -> Self {
        let ticks = if ticks.is_finite() {
            ticks.max(0.0)
        } else {
            0.0
        };
        self.shader_game_time = [(ticks % 24_000.0) as f32 / 24_000.0, 0.0, 0.0, 0.0];
        self
    }

    pub(crate) fn with_viewport_size(mut self, width: f32, height: f32) -> Self {
        self.viewport_size = [
            sanitize_viewport_extent(width),
            sanitize_viewport_extent(height),
            0.0,
            0.0,
        ];
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
            level_lighting: if self.minecraft_light1[1] < 0.0 {
                LevelLighting::Nether
            } else {
                LevelLighting::Default
            },
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

    #[cfg(test)]
    pub(crate) fn shader_lighting(self) -> ([f32; 3], [f32; 3]) {
        (
            self.minecraft_light0[0..3].try_into().unwrap(),
            self.minecraft_light1[0..3].try_into().unwrap(),
        )
    }

    #[cfg(test)]
    pub(crate) fn glint_offsets(self) -> [f32; 4] {
        self.glint_offsets
    }

    #[cfg(test)]
    pub(crate) fn shader_game_time(self) -> [f32; 4] {
        self.shader_game_time
    }

    #[cfg(test)]
    pub(crate) fn viewport_size(self) -> [f32; 4] {
        self.viewport_size
    }

    #[cfg(test)]
    pub(crate) fn view_proj(self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.view_proj)
    }

    pub(crate) fn sky_model_view_transform(self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.sky_model_view)
    }

    #[cfg(test)]
    pub(crate) fn projection(self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.projection)
    }

    #[cfg(test)]
    pub(crate) fn view_proj_view_offset_z(self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.view_proj_view_offset_z)
    }

    #[cfg(test)]
    pub(crate) fn view_proj_view_offset_z_forward(self) -> Mat4 {
        Mat4::from_cols_array_2d(&self.view_proj_view_offset_z_forward)
    }

    fn from_lightmap_environment(environment: LightmapEnvironment) -> Self {
        let environment = environment.sanitized();
        let (minecraft_light0, minecraft_light1) =
            lighting_entry_directions(environment.level_lighting.entry());
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
            minecraft_light0: vec3_to_vec4(minecraft_light0),
            minecraft_light1: vec3_to_vec4(minecraft_light1),
            glint_offsets: [0.0, 0.0, 0.0, 0.0],
            view_proj_view_offset_z: Mat4::IDENTITY.to_cols_array_2d(),
            view_proj_view_offset_z_forward: Mat4::IDENTITY.to_cols_array_2d(),
            projection: Mat4::IDENTITY.to_cols_array_2d(),
            sky_model_view: Mat4::IDENTITY.to_cols_array_2d(),
            shader_game_time: [0.0, 0.0, 0.0, 0.0],
            viewport_size: [1.0, 1.0, 0.0, 0.0],
        }
    }
}

fn sanitize_perspective_fov_degrees(fov_degrees: f32) -> f32 {
    if fov_degrees.is_finite() {
        fov_degrees.clamp(0.1, 179.0)
    } else {
        70.0
    }
}

fn camera_hurt_transform(effect: Option<CameraHurtEffect>) -> Mat4 {
    let Some(effect) = effect else {
        return Mat4::IDENTITY;
    };
    let mut transform = Mat4::IDENTITY;
    if effect.is_dead_or_dying && effect.death_time.is_finite() {
        let duration = effect.death_time.clamp(0.0, 20.0);
        let degrees = 40.0 - 8_000.0 / (duration + 200.0);
        transform *= Mat4::from_rotation_z(degrees.to_radians());
    }
    if !effect.hurt_time.is_finite()
        || effect.hurt_time < 0.0
        || !effect.hurt_duration.is_finite()
        || effect.hurt_duration <= 0.0
    {
        return transform;
    }
    let hurt = effect.hurt_time / effect.hurt_duration;
    let hurt = (hurt * hurt * hurt * hurt * std::f32::consts::PI).sin();
    let hurt_dir = if effect.hurt_dir.is_finite() {
        effect.hurt_dir
    } else {
        0.0
    };
    let strength = if effect.damage_tilt_strength.is_finite() {
        effect.damage_tilt_strength.clamp(0.0, 1.0)
    } else {
        1.0
    };
    transform *= Mat4::from_rotation_y((-hurt_dir).to_radians());
    transform *= Mat4::from_rotation_z((-hurt * 14.0 * strength).to_radians());
    transform *= Mat4::from_rotation_y(hurt_dir.to_radians());
    transform
}

fn camera_view_bob_transform(effect: Option<CameraViewBobEffect>) -> Mat4 {
    let Some(effect) = effect else {
        return Mat4::IDENTITY;
    };
    if !effect.backwards_interpolated_walk_distance.is_finite() || !effect.bob.is_finite() {
        return Mat4::IDENTITY;
    }
    let phase = effect.backwards_interpolated_walk_distance * std::f32::consts::PI;
    let sin = phase.sin();
    let cos = phase.cos();
    let x_rotation_degrees = ((phase - 0.2).cos() * effect.bob).abs() * 5.0;
    Mat4::from_translation(Vec3::new(
        sin * effect.bob * 0.5,
        -(cos * effect.bob).abs(),
        0.0,
    )) * Mat4::from_rotation_z((sin * effect.bob * 3.0).to_radians())
        * Mat4::from_rotation_x(x_rotation_degrees.to_radians())
}

fn camera_spinning_transform(effect: Option<CameraSpinningEffect>) -> Mat4 {
    let Some(effect) = effect else {
        return Mat4::IDENTITY;
    };
    if !effect.intensity.is_finite() || effect.intensity <= 0.0 || !effect.angle_degrees.is_finite()
    {
        return Mat4::IDENTITY;
    }
    let intensity = effect.intensity.clamp(0.0, 1.0);
    let mut skew = 5.0 / (intensity * intensity + 5.0) - intensity * 0.04;
    skew *= skew;
    let axis = Vec3::new(
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
    );
    let angle = effect.angle_degrees.to_radians();
    Mat4::from_axis_angle(axis, angle)
        * Mat4::from_scale(Vec3::new(1.0 / skew, 1.0, 1.0))
        * Mat4::from_axis_angle(axis, -angle)
}

pub(crate) fn vanilla_glint_texture_offsets(elapsed_millis: f64, glint_speed: f64) -> [f32; 2] {
    const GLINT_SPEED_MILLIS_MULTIPLIER: f64 = 8.0;
    const LAYER0_PERIOD_MILLIS: u64 = 110_000;
    const LAYER1_PERIOD_MILLIS: u64 = 30_000;

    let elapsed_millis = if elapsed_millis.is_finite() {
        elapsed_millis.max(0.0)
    } else {
        0.0
    };
    let glint_speed = if glint_speed.is_finite() {
        glint_speed.max(0.0)
    } else {
        VANILLA_DEFAULT_GLINT_SPEED
    };
    let millis = (elapsed_millis * glint_speed * GLINT_SPEED_MILLIS_MULTIPLIER) as u64;
    let layer0 = (millis % LAYER0_PERIOD_MILLIS) as f32 / LAYER0_PERIOD_MILLIS as f32;
    let layer1 = (millis % LAYER1_PERIOD_MILLIS) as f32 / LAYER1_PERIOD_MILLIS as f32;
    [-layer0, layer1]
}

fn perspective_view_offset_z_projection(projection: Mat4, view: Mat4) -> Mat4 {
    projection * Mat4::from_scale(Vec3::splat(VANILLA_VIEW_OFFSET_Z_PERSPECTIVE_SCALE)) * view
}

fn perspective_view_offset_z_forward_projection(projection: Mat4, view: Mat4) -> Mat4 {
    projection
        * Mat4::from_scale(Vec3::splat(VANILLA_VIEW_OFFSET_Z_FORWARD_PERSPECTIVE_SCALE))
        * view
}

fn orthographic_view_offset_z_projection(projection: Mat4, view: Mat4) -> Mat4 {
    projection
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.0,
            VANILLA_VIEW_OFFSET_Z_ORTHOGRAPHIC_TRANSLATE,
        ))
        * view
}

fn orthographic_view_offset_z_forward_projection(projection: Mat4, view: Mat4) -> Mat4 {
    projection
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.0,
            VANILLA_VIEW_OFFSET_Z_FORWARD_ORTHOGRAPHIC_TRANSLATE,
        ))
        * view
}

fn view_without_translation(view: Mat4) -> Mat4 {
    Mat4::from_cols(view.x_axis, view.y_axis, view.z_axis, Vec4::W)
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

fn sanitize_viewport_extent(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

fn rgb_to_vec4(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 0.0]
}

fn vec3_to_vec4(value: Vec3) -> [f32; 4] {
    [value.x, value.y, value.z, 0.0]
}

fn lighting_entry_directions(entry: LightingEntry) -> (Vec3, Vec3) {
    let diffuse0 = Vec3::new(0.2, 1.0, -0.7).normalize();
    let diffuse1 = Vec3::new(-0.2, 1.0, 0.7).normalize();
    match entry {
        LightingEntry::LevelDefault => (diffuse0, diffuse1),
        LightingEntry::LevelNether => (diffuse0, Vec3::new(-0.2, -1.0, 0.7).normalize()),
        LightingEntry::ItemsFlat => {
            let pose = Mat4::from_rotation_y(-std::f32::consts::PI / 8.0)
                * Mat4::from_rotation_x(std::f32::consts::PI * 3.0 / 4.0);
            (
                pose.transform_vector3(diffuse0).normalize(),
                pose.transform_vector3(diffuse1).normalize(),
            )
        }
        LightingEntry::Items3d => {
            let pose = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
                * Mat4::from_euler(EulerRot::YXZ, 1.0821041, 3.2375858, 0.0)
                * Mat4::from_euler(
                    EulerRot::YXZ,
                    -std::f32::consts::PI / 8.0,
                    std::f32::consts::PI * 3.0 / 4.0,
                    0.0,
                );
            (
                pose.transform_vector3(diffuse0).normalize(),
                pose.transform_vector3(diffuse1).normalize(),
            )
        }
        LightingEntry::EntityInUi => (
            Vec3::new(0.2, -1.0, 1.0).normalize(),
            Vec3::new(-0.2, -1.0, 0.0).normalize(),
        ),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainBounds {
    min: Vec3,
    max: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vec3_close(actual: [f32; 3], expected: Vec3) {
        let actual = Vec3::from_array(actual);
        assert!(
            actual.abs_diff_eq(expected, 1.0e-5),
            "expected {expected:?}, got {actual:?}"
        );
    }

    fn assert_vec2_close(actual: [f32; 2], expected: [f32; 2]) {
        assert!(
            (actual[0] - expected[0]).abs() < 1.0e-6 && (actual[1] - expected[1]).abs() < 1.0e-6,
            "expected {expected:?}, got {actual:?}"
        );
    }

    fn assert_mat4_close(actual: Mat4, expected: Mat4) {
        assert!(
            actual.abs_diff_eq(expected, 1.0e-5),
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn camera_frustum_accepts_front_intersections_and_rejects_behind_boxes() {
        let frustum = CameraFrustum::from_pose(
            CameraPose {
                position: [0.0; 3],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            },
            16.0 / 9.0,
            70.0,
        );
        assert!(frustum.intersects_aabb([-0.5, -0.5, 4.0], [0.5, 0.5, 5.0]));
        assert!(!frustum.intersects_aabb([-0.5, -0.5, -5.0], [0.5, 0.5, -4.0]));
        assert!(frustum.intersects_aabb([-100.0, -1.0, 1.0], [100.0, 1.0, 2.0]));
        assert!(frustum.intersects_aabb([f64::NAN, 0.0, 0.0], [1.0; 3]));
        assert_eq!(
            frustum.classify_aabb([-0.1, -0.1, 4.0], [0.1, 0.1, 4.2]),
            AabbFrustumIntersection::Inside
        );
        assert_eq!(
            frustum.classify_aabb([-100.0, -1.0, 1.0], [100.0, 1.0, 2.0]),
            AabbFrustumIntersection::Intersecting
        );
    }

    #[test]
    fn camera_billboard_axes_match_vanilla_camera_rotation_and_front_winding() {
        let south = CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }
        .billboard_axes();
        assert!(south.forward.abs_diff_eq(Vec3::Z, 1.0e-6));
        assert!(south.right.abs_diff_eq(Vec3::NEG_X, 1.0e-6));
        assert!(south.up.abs_diff_eq(Vec3::Y, 1.0e-6));
        assert!(south
            .right
            .cross(south.up)
            .abs_diff_eq(-south.forward, 1.0e-6));

        let west = CameraPose {
            y_rot: 90.0,
            ..CameraPose {
                position: [0.0; 3],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            }
        }
        .billboard_axes();
        assert!(west.forward.abs_diff_eq(Vec3::NEG_X, 1.0e-6));
        assert!(west.right.abs_diff_eq(Vec3::NEG_Z, 1.0e-6));
        assert!(west.up.abs_diff_eq(Vec3::Y, 1.0e-6));
    }

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
        assert_eq!(environment.level_lighting, LevelLighting::Default);
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
    fn camera_uniform_defaults_to_vanilla_level_lighting() {
        let (light0, light1) = CameraUniform::identity().shader_lighting();

        assert_vec3_close(light0, Vec3::new(0.2, 1.0, -0.7).normalize());
        assert_vec3_close(light1, Vec3::new(-0.2, 1.0, 0.7).normalize());
        assert_eq!(
            CameraUniform::from_bounds(
                TerrainBounds::from_points([Vec3::ZERO, Vec3::ONE]).unwrap(),
                1.0
            )
            .shader_lighting(),
            CameraUniform::identity().shader_lighting()
        );
    }

    #[test]
    fn camera_uniform_uses_nether_level_lighting_from_environment() {
        let (light0, light1) = CameraUniform::identity()
            .with_lightmap_environment(LightmapEnvironment {
                level_lighting: LevelLighting::Nether,
                ..LightmapEnvironment::default()
            })
            .shader_lighting();

        assert_vec3_close(light0, Vec3::new(0.2, 1.0, -0.7).normalize());
        assert_vec3_close(light1, Vec3::new(-0.2, -1.0, 0.7).normalize());
    }

    #[test]
    fn gui_ortho_uses_vanilla_items_3d_lighting_entry() {
        let level = CameraUniform::identity().shader_lighting();
        let gui = CameraUniform::gui_ortho(320.0, 240.0).shader_lighting();
        let expected = CameraUniform::identity()
            .with_lighting_entry(LightingEntry::Items3d)
            .shader_lighting();

        assert_eq!(gui, expected);
        assert_ne!(gui, level);
    }

    #[test]
    fn hud_entity_preview_pip_folds_vanilla_pip_pose_chain_into_gui_ortho() {
        // Fixed smithing-screen preview: 40x60 texture (vanilla `SmithingScreen` rect 121..161 x
        // 20..80 at guiScale 1), scale 25, translation (0, 1, 0), rotation rotX(0.43633232) *
        // rotZ(PI). Expect exactly `ortho(0, w, h, 0, -1000, 1000) * T(w/2, h/2, 0) * S(s, s, -s)
        // * T(translation) * R(rotation)` — `PictureInPictureRenderer.prepare` +
        // `GuiEntityRenderer.renderToTexture`, with `getTranslateY == height / 2`.
        let rotation = glam::Quat::from_rotation_x(0.43633232)
            * glam::Quat::from_rotation_z(std::f32::consts::PI);
        let uniform = CameraUniform::hud_entity_preview_pip(
            40.0,
            60.0,
            25.0,
            [0.0, 1.0, 0.0],
            rotation.to_array(),
        );

        let projection = Mat4::orthographic_rh(0.0, 40.0, 60.0, 0.0, -1000.0, 1000.0);
        let model = Mat4::from_translation(Vec3::new(20.0, 30.0, 0.0))
            * Mat4::from_scale(Vec3::new(25.0, 25.0, -25.0))
            * Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0))
            * Mat4::from_quat(rotation);
        assert_mat4_close(uniform.view_proj(), projection * model);
        assert_mat4_close(uniform.projection(), projection);
        // The vanilla ortho layering transforms fold the same model chain.
        assert_mat4_close(
            uniform.view_proj_view_offset_z(),
            orthographic_view_offset_z_projection(projection, model),
        );

        // `GuiEntityRenderer` renders under `Lighting.Entry.ENTITY_IN_UI`, with fog disabled.
        assert_eq!(
            uniform.shader_lighting(),
            CameraUniform::identity()
                .with_lighting_entry(LightingEntry::EntityInUi)
                .shader_lighting()
        );
        assert_eq!(uniform.fog_environment(), FogEnvironment::disabled());

        // The entity's feet anchor (model origin) projects to the texture center shifted by the
        // scaled translation: the vanilla pose chain applies `state.translation` before
        // `state.rotation`, in the y-down scaled GUI space, so +y translation moves the origin
        // down — y = h/2 + scale * translation.y (the rotZ(PI) flip then draws the y-up model
        // body upward from that anchor). x stays centered.
        let clip = uniform.view_proj() * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let ndc = clip / clip.w;
        let px = (ndc.x * 0.5 + 0.5) * 40.0;
        let py = (0.5 - ndc.y * 0.5) * 60.0;
        assert!((px - 20.0).abs() < 1.0e-3, "origin x pixel, got {px}");
        assert!((py - 55.0).abs() < 1.0e-3, "origin y pixel, got {py}");
    }

    #[test]
    fn hud_entity_preview_pip_normalizes_degenerate_rotation() {
        let uniform =
            CameraUniform::hud_entity_preview_pip(40.0, 60.0, 25.0, [0.0, 0.0, 0.0], [0.0; 4]);
        let expected = CameraUniform::hud_entity_preview_pip(
            40.0,
            60.0,
            25.0,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        );
        assert_mat4_close(uniform.view_proj(), expected.view_proj());
    }

    #[test]
    fn camera_hurt_transform_matches_vanilla_bob_hurt_order() {
        let effect = CameraHurtEffect {
            hurt_time: 5.0,
            hurt_duration: 10.0,
            hurt_dir: 35.0,
            death_time: 8.0,
            is_dead_or_dying: true,
            damage_tilt_strength: 0.5,
        };
        let death_degrees: f32 = 40.0 - 8_000.0 / 208.0;
        let hurt = ((0.5_f32).powi(4) * std::f32::consts::PI).sin();
        let expected = Mat4::from_rotation_z(death_degrees.to_radians())
            * Mat4::from_rotation_y((-35.0_f32).to_radians())
            * Mat4::from_rotation_z((-hurt * 14.0 * 0.5).to_radians())
            * Mat4::from_rotation_y(35.0_f32.to_radians());
        assert_mat4_close(camera_hurt_transform(Some(effect)), expected);
    }

    #[test]
    fn camera_uniform_applies_hurt_after_projection_without_changing_view() {
        let pose = CameraPose {
            position: [1.0, 64.0, -2.0],
            y_rot: 25.0,
            x_rot: -10.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let base = CameraUniform::from_pose_with_fov(pose, 16.0 / 9.0, 70.0);
        let effect = CameraHurtEffect {
            hurt_time: 5.0,
            hurt_duration: 10.0,
            hurt_dir: 20.0,
            death_time: 0.0,
            is_dead_or_dying: false,
            damage_tilt_strength: 1.0,
        };
        let hurt = CameraUniform::from_pose_with_fov_and_hurt(pose, 16.0 / 9.0, 70.0, Some(effect));
        let transform = camera_hurt_transform(Some(effect));

        assert_mat4_close(hurt.projection(), base.projection() * transform);
        assert_mat4_close(
            hurt.sky_model_view_transform(),
            base.sky_model_view_transform(),
        );
        assert_mat4_close(
            hurt.view_proj(),
            base.projection() * transform * base.projection().inverse() * base.view_proj(),
        );

        let disabled = CameraUniform::from_pose_with_fov_and_hurt(
            pose,
            16.0 / 9.0,
            70.0,
            Some(CameraHurtEffect {
                damage_tilt_strength: 0.0,
                ..effect
            }),
        );
        assert_mat4_close(disabled.projection(), base.projection());
    }

    #[test]
    fn camera_spinning_transform_matches_vanilla_projection_order() {
        let effect = CameraSpinningEffect {
            intensity: 0.5,
            angle_degrees: 35.0,
        };
        let mut skew = 5.0 / (0.5 * 0.5 + 5.0) - 0.5 * 0.04;
        skew *= skew;
        let axis = Vec3::new(
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
        );
        let angle = 35.0_f32.to_radians();
        let expected = Mat4::from_axis_angle(axis, angle)
            * Mat4::from_scale(Vec3::new(1.0 / skew, 1.0, 1.0))
            * Mat4::from_axis_angle(axis, -angle);
        assert_mat4_close(camera_spinning_transform(Some(effect)), expected);
    }

    #[test]
    fn camera_view_bob_transform_matches_vanilla_translate_and_rotation_order() {
        let effect = CameraViewBobEffect {
            backwards_interpolated_walk_distance: 0.25,
            bob: 0.08,
        };
        let phase = 0.25 * std::f32::consts::PI;
        let sin = phase.sin();
        let cos = phase.cos();
        let expected =
            Mat4::from_translation(Vec3::new(sin * 0.08 * 0.5, -(cos * 0.08).abs(), 0.0))
                * Mat4::from_rotation_z((sin * 0.08 * 3.0).to_radians())
                * Mat4::from_rotation_x((((phase - 0.2).cos() * 0.08).abs() * 5.0).to_radians());

        assert_mat4_close(camera_view_bob_transform(Some(effect)), expected);
        assert_mat4_close(camera_view_bob_transform(None), Mat4::IDENTITY);
        assert_mat4_close(
            camera_view_bob_transform(Some(CameraViewBobEffect {
                backwards_interpolated_walk_distance: f32::NAN,
                bob: 0.08,
            })),
            Mat4::IDENTITY,
        );
    }

    #[test]
    fn camera_view_bob_uniform_applies_between_hurt_and_spinning_to_world_and_hud() {
        let pose = CameraPose {
            position: [1.0, 64.0, -2.0],
            y_rot: 25.0,
            x_rot: -10.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let base = CameraUniform::from_pose_with_fov(pose, 16.0 / 9.0, 70.0);
        let base_hud = CameraUniform::from_hud_pose(pose, 16.0 / 9.0, 60.0, None);
        let hurt = CameraHurtEffect {
            hurt_time: 5.0,
            hurt_duration: 10.0,
            hurt_dir: 20.0,
            death_time: 0.0,
            is_dead_or_dying: false,
            damage_tilt_strength: 1.0,
        };
        let view_bob = CameraViewBobEffect {
            backwards_interpolated_walk_distance: -1.25,
            bob: 0.06,
        };
        let spinning = CameraSpinningEffect {
            intensity: 0.75,
            angle_degrees: 42.0,
        };
        let world = CameraUniform::from_pose_with_camera_effects(
            pose,
            16.0 / 9.0,
            70.0,
            Some(hurt),
            Some(view_bob),
            Some(spinning),
        );
        let hud = CameraUniform::from_hud_pose_with_view_bob(
            pose,
            16.0 / 9.0,
            60.0,
            Some(hurt),
            Some(view_bob),
        );

        assert_mat4_close(
            world.projection(),
            base.projection()
                * camera_hurt_transform(Some(hurt))
                * camera_view_bob_transform(Some(view_bob))
                * camera_spinning_transform(Some(spinning)),
        );
        assert_mat4_close(
            hud.projection(),
            base_hud.projection()
                * camera_hurt_transform(Some(hurt))
                * camera_view_bob_transform(Some(view_bob)),
        );
        assert_mat4_close(
            world.sky_model_view_transform(),
            base.sky_model_view_transform(),
        );
    }

    #[test]
    fn camera_uniform_applies_spinning_after_hurt_without_changing_view() {
        let pose = CameraPose {
            position: [1.0, 64.0, -2.0],
            y_rot: 25.0,
            x_rot: -10.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let base = CameraUniform::from_pose_with_fov(pose, 16.0 / 9.0, 70.0);
        let hurt = CameraHurtEffect {
            hurt_time: 5.0,
            hurt_duration: 10.0,
            hurt_dir: 20.0,
            death_time: 0.0,
            is_dead_or_dying: false,
            damage_tilt_strength: 1.0,
        };
        let spinning = CameraSpinningEffect {
            intensity: 0.75,
            angle_degrees: 42.0,
        };
        let uniform = CameraUniform::from_pose_with_effects(
            pose,
            16.0 / 9.0,
            70.0,
            Some(hurt),
            Some(spinning),
        );
        let transform =
            camera_hurt_transform(Some(hurt)) * camera_spinning_transform(Some(spinning));

        assert_mat4_close(uniform.projection(), base.projection() * transform);
        assert_mat4_close(
            uniform.sky_model_view_transform(),
            base.sky_model_view_transform(),
        );
    }

    #[test]
    fn vanilla_glint_texture_offsets_match_texture_transform_periods() {
        assert_vec2_close(
            vanilla_glint_texture_offsets(0.0, VANILLA_DEFAULT_GLINT_SPEED),
            [0.0, 0.0],
        );
        assert_vec2_close(
            vanilla_glint_texture_offsets(1000.0, VANILLA_DEFAULT_GLINT_SPEED),
            [-4000.0 / 110_000.0, 4000.0 / 30_000.0],
        );
        assert_vec2_close(
            vanilla_glint_texture_offsets(27_500.0, VANILLA_DEFAULT_GLINT_SPEED),
            [0.0, 20_000.0 / 30_000.0],
        );
    }

    #[test]
    fn camera_uniform_stores_vanilla_glint_offsets() {
        assert_eq!(CameraUniform::identity().glint_offsets(), [0.0; 4]);
        assert_eq!(
            CameraUniform::identity()
                .with_glint_texture_time(1000.0, VANILLA_DEFAULT_GLINT_SPEED)
                .glint_offsets(),
            [-4000.0 / 110_000.0, 4000.0 / 30_000.0, 0.0, 0.0]
        );
    }

    #[test]
    fn camera_uniform_stores_vanilla_shader_game_time() {
        assert_eq!(CameraUniform::identity().shader_game_time(), [0.0; 4]);
        assert_eq!(
            CameraUniform::identity()
                .with_shader_game_time_ticks(24_006.0)
                .shader_game_time(),
            [6.0 / 24_000.0, 0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn camera_uniform_stores_viewport_size_for_line_widths() {
        assert_eq!(
            CameraUniform::identity().viewport_size(),
            [1.0, 1.0, 0.0, 0.0]
        );
        assert_eq!(
            CameraUniform::identity()
                .with_viewport_size(1280.0, 720.0)
                .viewport_size(),
            [1280.0, 720.0, 0.0, 0.0]
        );
        assert_eq!(
            CameraUniform::identity()
                .with_viewport_size(0.0, f32::NAN)
                .viewport_size(),
            [1.0, 1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn camera_uniform_stores_perspective_view_offset_z_layering() {
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        };
        let projection = Mat4::perspective_rh(70.0_f32.to_radians(), 16.0 / 9.0, 0.05, 2048.0);
        let view = Mat4::look_at_rh(
            Vec3::new(0.0, 64.0, 0.0),
            Vec3::new(0.0, 64.0, 1.0),
            Vec3::Y,
        );
        let expected = projection
            * Mat4::from_scale(Vec3::splat(VANILLA_VIEW_OFFSET_Z_PERSPECTIVE_SCALE))
            * view;
        let uniform = CameraUniform::from_pose(pose, 16.0 / 9.0);

        assert_mat4_close(uniform.view_proj_view_offset_z(), expected);
        assert!(!uniform
            .view_proj()
            .abs_diff_eq(uniform.view_proj_view_offset_z(), 1.0e-8));
    }

    #[test]
    fn camera_uniform_uses_dynamic_perspective_fov() {
        let pose = CameraPose {
            position: [0.0, 0.0, 0.0],
            eye_height: 1.62,
            y_rot: 0.0,
            x_rot: 0.0,
        };
        let uniform = CameraUniform::from_pose_with_fov(pose, 16.0 / 9.0, 35.0);
        let expected = Mat4::perspective_rh(35.0_f32.to_radians(), 16.0 / 9.0, 0.05, 2048.0);
        assert_mat4_close(uniform.projection(), expected);
    }

    #[test]
    fn camera_fov_projection_admits_vanilla_max_modifier_and_separate_hud_far_plane() {
        let pose = CameraPose {
            position: [0.0, 0.0, 0.0],
            eye_height: 1.62,
            y_rot: 0.0,
            x_rot: 0.0,
        };
        let world = CameraUniform::from_pose_with_fov(pose, 16.0 / 9.0, 165.0);
        let expected_world = Mat4::perspective_rh(165.0_f32.to_radians(), 16.0 / 9.0, 0.05, 2048.0);
        assert_mat4_close(world.projection(), expected_world);
        let eye = Vec3::new(0.0, 1.62, 0.0);
        let expected_view = Mat4::look_at_rh(eye, eye + Vec3::Z, Vec3::Y);
        let frustum = CameraFrustum::from_pose(pose, 16.0 / 9.0, 165.0);
        assert_mat4_close(frustum.view_projection, expected_world * expected_view);

        let hud = CameraUniform::from_hud_pose(pose, 16.0 / 9.0, 60.0, None);
        let expected_hud = Mat4::perspective_rh(60.0_f32.to_radians(), 16.0 / 9.0, 0.05, 100.0);
        assert_mat4_close(hud.projection(), expected_hud);
    }

    #[test]
    fn camera_uniform_stores_perspective_view_offset_z_forward_layering() {
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        };
        let projection = Mat4::perspective_rh(70.0_f32.to_radians(), 16.0 / 9.0, 0.05, 2048.0);
        let view = Mat4::look_at_rh(
            Vec3::new(0.0, 64.0, 0.0),
            Vec3::new(0.0, 64.0, 1.0),
            Vec3::Y,
        );
        let expected = projection
            * Mat4::from_scale(Vec3::splat(VANILLA_VIEW_OFFSET_Z_FORWARD_PERSPECTIVE_SCALE))
            * view;
        let uniform = CameraUniform::from_pose(pose, 16.0 / 9.0);

        assert_mat4_close(uniform.view_proj_view_offset_z_forward(), expected);
        assert!(!uniform
            .view_proj_view_offset_z()
            .abs_diff_eq(uniform.view_proj_view_offset_z_forward(), 1.0e-8));
    }

    #[test]
    fn camera_uniform_stores_orthographic_view_offset_z_layering() {
        let projection = Mat4::orthographic_rh(0.0, 320.0, 240.0, 0.0, -1000.0, 1000.0);
        let expected = projection
            * Mat4::from_translation(Vec3::new(
                0.0,
                0.0,
                VANILLA_VIEW_OFFSET_Z_ORTHOGRAPHIC_TRANSLATE,
            ));
        let uniform = CameraUniform::gui_ortho(320.0, 240.0);

        assert_mat4_close(uniform.view_proj_view_offset_z(), expected);
        assert!(!uniform
            .view_proj()
            .abs_diff_eq(uniform.view_proj_view_offset_z(), 1.0e-8));
    }

    #[test]
    fn camera_uniform_stores_orthographic_view_offset_z_forward_layering() {
        let projection = Mat4::orthographic_rh(0.0, 320.0, 240.0, 0.0, -1000.0, 1000.0);
        let expected = projection
            * Mat4::from_translation(Vec3::new(
                0.0,
                0.0,
                VANILLA_VIEW_OFFSET_Z_FORWARD_ORTHOGRAPHIC_TRANSLATE,
            ));
        let uniform = CameraUniform::gui_ortho(320.0, 240.0);

        assert_mat4_close(uniform.view_proj_view_offset_z_forward(), expected);
        assert!(!uniform
            .view_proj_view_offset_z()
            .abs_diff_eq(uniform.view_proj_view_offset_z_forward(), 1.0e-8));
    }

    #[test]
    fn camera_uniform_diffuse_lighting_entries_match_vanilla_directions() {
        let level = CameraUniform::identity().shader_lighting();
        let items_flat = CameraUniform::identity()
            .with_lighting_entry(LightingEntry::ItemsFlat)
            .shader_lighting();
        let items_3d = CameraUniform::identity()
            .with_lighting_entry(LightingEntry::Items3d)
            .shader_lighting();
        let entity_in_ui = CameraUniform::identity()
            .with_lighting_entry(LightingEntry::EntityInUi)
            .shader_lighting();

        assert_vec3_close(level.0, Vec3::new(0.16169042, 0.80845207, -0.5659165));
        assert_vec3_close(level.1, Vec3::new(-0.16169042, 0.80845207, 0.5659165));
        assert_vec3_close(
            items_flat.0,
            Vec3::new(-0.22251898, -0.17149858, 0.95972574),
        );
        assert_vec3_close(items_flat.1, Vec3::new(-0.21501213, -0.9718253, 0.09656779));
        assert_vec3_close(items_3d.0, Vec3::new(-0.9334393, -0.26269472, -0.24430019));
        assert_vec3_close(items_3d.1, Vec3::new(-0.10357137, -0.97660685, 0.18844643));
        assert_vec3_close(entity_in_ui.0, Vec3::new(0.2, -1.0, 1.0).normalize());
        assert_vec3_close(entity_in_ui.1, Vec3::new(-0.2, -1.0, 0.0).normalize());
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
    fn camera_uniform_splits_sky_projection_and_model_view_without_changing_projection() {
        let pose = CameraPose {
            position: [4.0, 64.0, -8.0],
            y_rot: 35.0,
            x_rot: -12.0,
            eye_height: 1.5,
        };
        let uniform = CameraUniform::from_pose(pose, 16.0 / 9.0);
        let sky_position = Vec3::new(32.0, 100.0, -24.0);
        let old_projection = uniform.view_proj()
            * (sky_position + Vec3::from_array(uniform.camera_position())).extend(1.0);
        let split_projection =
            uniform.projection() * uniform.sky_model_view_transform() * sky_position.extend(1.0);

        assert!(
            old_projection.abs_diff_eq(split_projection, 1.0e-4),
            "expected split sky projection {split_projection:?}, got {old_projection:?}"
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
            level_lighting: LevelLighting::Nether,
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
        assert_eq!(sanitized.level_lighting, LevelLighting::Nether);
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

    pub(crate) fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn extent(self) -> Vec3 {
        (self.max - self.min).max(Vec3::splat(1.0))
    }
}
