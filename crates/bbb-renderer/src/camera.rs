use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

use crate::terrain;

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
}

impl CameraUniform {
    pub(crate) fn identity() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
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
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainBounds {
    min: Vec3,
    max: Vec3,
}

impl TerrainBounds {
    pub(crate) fn from_vertices(vertices: &[terrain::TerrainVertex]) -> Option<Self> {
        let mut vertices = vertices.iter();
        let first = vertices.next()?;
        let mut bounds = Self {
            min: Vec3::from_array(first.position),
            max: Vec3::from_array(first.position),
        };
        for vertex in vertices {
            bounds.include_point(Vec3::from_array(vertex.position));
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
