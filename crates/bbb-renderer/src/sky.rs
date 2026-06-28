use wgpu::util::DeviceExt;

use crate::gpu::DEPTH_FORMAT;

const SKY_DISC_RADIUS: f32 = 512.0;
const SKY_DISC_Y: f32 = 16.0;
const SUNRISE_STEPS: usize = 16;
const SUNRISE_CENTER_Y: f32 = 100.0;
const SUNRISE_RING_RADIUS: f32 = 120.0;
const SUNRISE_RING_DEPTH: f32 = 40.0;
const SUNRISE_ALPHA_EPSILON: f32 = 0.001;

const SKY_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>,
    sky_light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_visibility_ends: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world_position = input.position + camera.camera_position.xyz;
    out.position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyEnvironment {
    pub color: [f32; 4],
    pub sunrise_sunset_color: [f32; 4],
    pub sun_angle_radians: f32,
}

impl Default for SkyEnvironment {
    fn default() -> Self {
        Self::disabled()
    }
}

impl SkyEnvironment {
    pub fn disabled() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 0.0],
            sunrise_sunset_color: [0.0, 0.0, 0.0, 0.0],
            sun_angle_radians: 0.0,
        }
    }

    pub fn from_rgb(color: [f32; 3]) -> Self {
        Self {
            color: [color[0], color[1], color[2], 1.0],
            sunrise_sunset_color: [0.0, 0.0, 0.0, 0.0],
            sun_angle_radians: 0.0,
        }
        .sanitized()
    }

    pub fn with_sunrise_sunset(mut self, color: [f32; 4], sun_angle_radians: f32) -> Self {
        self.sunrise_sunset_color = color;
        self.sun_angle_radians = sun_angle_radians;
        self.sanitized()
    }

    pub fn sanitized(self) -> Self {
        Self {
            color: [
                sanitize_unit(self.color[0]),
                sanitize_unit(self.color[1]),
                sanitize_unit(self.color[2]),
                sanitize_unit(self.color[3]),
            ],
            sunrise_sunset_color: [
                sanitize_unit(self.sunrise_sunset_color[0]),
                sanitize_unit(self.sunrise_sunset_color[1]),
                sanitize_unit(self.sunrise_sunset_color[2]),
                sanitize_unit(self.sunrise_sunset_color[3]),
            ],
            sun_angle_radians: sanitize_radians(self.sun_angle_radians),
        }
    }

    pub fn is_visible(self) -> bool {
        self.sanitized().color[3] > 0.0
    }

    pub fn sunrise_sunset_visible(self) -> bool {
        self.sanitized().sunrise_sunset_color[3] > SUNRISE_ALPHA_EPSILON
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyVertex {
    position: [f32; 3],
    color: [f32; 4],
}

pub(super) struct SkyDiscGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
}

pub(super) fn create_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-sky-disc-shader"),
        source: wgpu::ShaderSource::Wgsl(SKY_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-sky-disc-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-sky-disc-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SkyVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

pub(super) fn create_sky_disc_gpu(
    device: &wgpu::Device,
    environment: SkyEnvironment,
) -> Option<SkyDiscGpu> {
    let environment = environment.sanitized();
    if !environment.is_visible() && !environment.sunrise_sunset_visible() {
        return None;
    }
    let vertices = sky_vertices(environment);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-sky-disc-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    Some(SkyDiscGpu {
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    })
}

fn sky_vertices(environment: SkyEnvironment) -> Vec<SkyVertex> {
    let mut vertices = Vec::new();
    if environment.is_visible() {
        vertices.extend(sky_disc_vertices(environment.color));
    }
    vertices.extend(sunrise_sunset_vertices(
        environment.sunrise_sunset_color,
        environment.sun_angle_radians,
    ));
    vertices
}

fn sky_disc_vertices(color: [f32; 4]) -> Vec<SkyVertex> {
    let mut ring = Vec::with_capacity(9);
    for degrees in (-180..=180).step_by(45) {
        let radians = (degrees as f32).to_radians();
        ring.push([
            SKY_DISC_RADIUS * radians.cos(),
            SKY_DISC_Y,
            SKY_DISC_RADIUS * radians.sin(),
        ]);
    }

    let center = [0.0, SKY_DISC_Y, 0.0];
    let mut vertices = Vec::with_capacity((ring.len() - 1) * 3);
    for edge in ring.windows(2) {
        vertices.push(SkyVertex {
            position: center,
            color,
        });
        vertices.push(SkyVertex {
            position: edge[0],
            color,
        });
        vertices.push(SkyVertex {
            position: edge[1],
            color,
        });
    }
    vertices
}

fn sunrise_sunset_vertices(color: [f32; 4], sun_angle_radians: f32) -> Vec<SkyVertex> {
    let fan = sunrise_sunset_fan_vertices(color, sun_angle_radians);
    if fan.is_empty() {
        return Vec::new();
    }

    let mut vertices = Vec::with_capacity(SUNRISE_STEPS * 3);
    for edge in fan[1..].windows(2) {
        vertices.push(fan[0]);
        vertices.push(edge[0]);
        vertices.push(edge[1]);
    }
    vertices
}

fn sunrise_sunset_fan_vertices(color: [f32; 4], sun_angle_radians: f32) -> Vec<SkyVertex> {
    let color = [
        sanitize_unit(color[0]),
        sanitize_unit(color[1]),
        sanitize_unit(color[2]),
        sanitize_unit(color[3]),
    ];
    if color[3] <= SUNRISE_ALPHA_EPSILON {
        return Vec::new();
    }

    let mut vertices = Vec::with_capacity(SUNRISE_STEPS + 2);
    vertices.push(SkyVertex {
        position: sunrise_sunset_position(
            [0.0, SUNRISE_CENTER_Y, 0.0],
            sun_angle_radians,
            color[3],
        ),
        color,
    });

    for index in 0..=SUNRISE_STEPS {
        let angle = index as f32 * std::f32::consts::TAU / SUNRISE_STEPS as f32;
        let base = [
            angle.sin() * SUNRISE_RING_RADIUS,
            angle.cos() * SUNRISE_RING_RADIUS,
            -angle.cos() * SUNRISE_RING_DEPTH,
        ];
        vertices.push(SkyVertex {
            position: sunrise_sunset_position(base, sun_angle_radians, color[3]),
            color: [color[0], color[1], color[2], 0.0],
        });
    }

    vertices
}

fn sunrise_sunset_position(
    position: [f32; 3],
    sun_angle_radians: f32,
    sunrise_alpha: f32,
) -> [f32; 3] {
    let [x, y, z] = position;
    let z = z * sunrise_alpha;
    let z_rotation = if sun_angle_radians.sin() < 0.0 {
        std::f32::consts::PI
    } else {
        0.0
    } + std::f32::consts::FRAC_PI_2;
    let sin_z = z_rotation.sin();
    let cos_z = z_rotation.cos();
    let rotated_x = x * cos_z - y * sin_z;
    let rotated_y = x * sin_z + y * cos_z;

    // Vanilla applies an X+90 degree pose before drawing the sunrise/sunset fan.
    [rotated_x, -z, rotated_y]
}

fn sanitize_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn sanitize_radians(value: f32) -> f32 {
    if value.is_finite() {
        value.rem_euclid(std::f32::consts::TAU)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_environment_sanitizes_color_channels() {
        let environment = SkyEnvironment {
            color: [1.5, -1.0, f32::NAN, 2.0],
            sunrise_sunset_color: [-1.0, 0.5, 2.0, f32::NAN],
            sun_angle_radians: f32::INFINITY,
        }
        .sanitized();

        assert_eq!(environment.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(environment.sunrise_sunset_color, [0.0, 0.5, 1.0, 0.0]);
        assert_eq!(environment.sun_angle_radians, 0.0);
        assert!(environment.is_visible());
        assert!(!SkyEnvironment::disabled().is_visible());
    }

    #[test]
    fn sky_disc_vertices_match_vanilla_top_disc_shape() {
        let color = [0.25, 0.5, 0.75, 1.0];
        let vertices = sky_disc_vertices(color);

        assert_eq!(vertices.len(), 24);
        assert_eq!(vertices[0].position, [0.0, SKY_DISC_Y, 0.0]);
        assert_eq!(vertices[0].color, color);
        assert!((vertices[1].position[0] + SKY_DISC_RADIUS).abs() < 1e-4);
        assert_eq!(vertices[1].position[1], SKY_DISC_Y);
        assert!(vertices[1].position[2].abs() < 1e-4);
        assert!((vertices[23].position[0] + SKY_DISC_RADIUS).abs() < 1e-4);
        assert_eq!(vertices[23].position[1], SKY_DISC_Y);
        assert!(vertices[23].position[2].abs() < 1e-4);
    }

    #[test]
    fn sunrise_sunset_vertices_match_vanilla_fan_shape() {
        let color = [1.0, 0.5, 0.25, 0.5];
        let vertices = sunrise_sunset_fan_vertices(color, 0.0);

        assert_eq!(vertices.len(), SUNRISE_STEPS + 2);
        assert_eq!(vertices[0].color, color);
        assert_eq!(vertices[1].color, [1.0, 0.5, 0.25, 0.0]);
        assert_close(vertices[0].position[0], -SUNRISE_CENTER_Y);
        assert_close(vertices[0].position[1], 0.0);
        assert_close(vertices[0].position[2], 0.0);
        assert_close(vertices[1].position[0], -SUNRISE_RING_RADIUS);
        assert_close(vertices[1].position[1], SUNRISE_RING_DEPTH * color[3]);
        assert_close(vertices[1].position[2], 0.0);
    }

    #[test]
    fn sunrise_sunset_vertices_expand_fan_for_triangle_list_pipeline() {
        let vertices = sunrise_sunset_vertices([1.0, 0.5, 0.25, 0.5], 0.0);

        assert_eq!(vertices.len(), SUNRISE_STEPS * 3);
        assert_eq!(vertices[0].color, [1.0, 0.5, 0.25, 0.5]);
        assert_eq!(vertices[1].color, [1.0, 0.5, 0.25, 0.0]);
        assert_eq!(vertices[2].color, [1.0, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn sky_vertices_append_sunrise_sunset_after_top_disc() {
        let environment = SkyEnvironment::from_rgb([0.25, 0.5, 0.75])
            .with_sunrise_sunset([1.0, 0.25, 0.0, 0.75], 0.0);
        let vertices = sky_vertices(environment);

        assert_eq!(vertices.len(), 24 + SUNRISE_STEPS * 3);
        assert_eq!(vertices[0].color, environment.color);
        assert_eq!(vertices[24].color, environment.sunrise_sunset_color);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-4,
            "actual {actual} != expected {expected}"
        );
    }
}
