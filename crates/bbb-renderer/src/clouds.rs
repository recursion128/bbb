use wgpu::util::DeviceExt;

use crate::gpu::DEPTH_FORMAT;

pub const VANILLA_DEFAULT_CLOUD_COLOR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
pub const VANILLA_DEFAULT_CLOUD_HEIGHT: f32 = 192.33;

const CLOUD_PRESENTATION_HALF_EXTENT: f32 = 2048.0;

const CLOUD_SHADER: &str = r#"
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
    @location(1) fog_position: vec3<f32>,
};

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world_position = vec3<f32>(
        input.position.x + camera.camera_position.x,
        input.position.y,
        input.position.z + camera.camera_position.z,
    );
    out.position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.color = input.color;
    out.fog_position = world_position - camera.camera_position.xyz;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let cloud_end = camera.fog_visibility_ends.y;
    if (cloud_end <= 0.0) {
        discard;
    }
    var color = input.color;
    color.a *= 1.0 - linear_fog_value(length(input.fog_position), 0.0, cloud_end);
    return color;
}
"#;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudEnvironment {
    pub color: [f32; 4],
    pub height: f32,
}

impl Default for CloudEnvironment {
    fn default() -> Self {
        Self::disabled()
    }
}

impl CloudEnvironment {
    pub fn disabled() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 0.0],
            height: VANILLA_DEFAULT_CLOUD_HEIGHT,
        }
    }

    pub fn overworld_default() -> Self {
        Self {
            color: VANILLA_DEFAULT_CLOUD_COLOR,
            height: VANILLA_DEFAULT_CLOUD_HEIGHT,
        }
    }

    pub fn with_color_and_height(color: [f32; 4], height: f32) -> Self {
        Self { color, height }.sanitized()
    }

    pub fn sanitized(self) -> Self {
        Self {
            color: [
                sanitize_unit(self.color[0]),
                sanitize_unit(self.color[1]),
                sanitize_unit(self.color[2]),
                sanitize_unit(self.color[3]),
            ],
            height: sanitize_height(self.height),
        }
    }

    pub fn is_visible(self) -> bool {
        self.sanitized().color[3] > 0.0
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct CloudVertex {
    position: [f32; 3],
    color: [f32; 4],
}

pub(super) struct CloudGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
}

pub(super) fn create_cloud_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-cloud-shader"),
        source: wgpu::ShaderSource::Wgsl(CLOUD_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-cloud-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-cloud-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<CloudVertex>() as wgpu::BufferAddress,
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

pub(super) fn create_cloud_gpu(
    device: &wgpu::Device,
    environment: CloudEnvironment,
) -> Option<CloudGpu> {
    let vertices = cloud_vertices(environment);
    if vertices.is_empty() {
        return None;
    }
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-cloud-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    Some(CloudGpu {
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    })
}

fn cloud_vertices(environment: CloudEnvironment) -> Vec<CloudVertex> {
    let environment = environment.sanitized();
    if !environment.is_visible() {
        return Vec::new();
    }

    let y = environment.height;
    let extent = CLOUD_PRESENTATION_HALF_EXTENT;
    let quad = [
        CloudVertex {
            position: [-extent, y, -extent],
            color: environment.color,
        },
        CloudVertex {
            position: [extent, y, -extent],
            color: environment.color,
        },
        CloudVertex {
            position: [extent, y, extent],
            color: environment.color,
        },
        CloudVertex {
            position: [-extent, y, extent],
            color: environment.color,
        },
    ];
    vec![quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]
}

fn sanitize_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn sanitize_height(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        VANILLA_DEFAULT_CLOUD_HEIGHT
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cloud_vertices, CloudEnvironment, CLOUD_PRESENTATION_HALF_EXTENT, CLOUD_SHADER,
        VANILLA_DEFAULT_CLOUD_COLOR, VANILLA_DEFAULT_CLOUD_HEIGHT,
    };

    #[test]
    fn cloud_environment_uses_vanilla_overworld_defaults() {
        let environment = CloudEnvironment::overworld_default();

        assert_eq!(environment.color, VANILLA_DEFAULT_CLOUD_COLOR);
        assert_eq!(environment.height, VANILLA_DEFAULT_CLOUD_HEIGHT);
        assert!(environment.is_visible());
    }

    #[test]
    fn cloud_vertices_build_camera_centered_vanilla_height_plane() {
        let vertices = cloud_vertices(CloudEnvironment::overworld_default());

        assert_eq!(vertices.len(), 6);
        assert_eq!(
            vertices[0].position,
            [
                -CLOUD_PRESENTATION_HALF_EXTENT,
                VANILLA_DEFAULT_CLOUD_HEIGHT,
                -CLOUD_PRESENTATION_HALF_EXTENT,
            ]
        );
        assert_eq!(
            vertices[2].position,
            [
                CLOUD_PRESENTATION_HALF_EXTENT,
                VANILLA_DEFAULT_CLOUD_HEIGHT,
                CLOUD_PRESENTATION_HALF_EXTENT,
            ]
        );
        assert_eq!(vertices[0].color, VANILLA_DEFAULT_CLOUD_COLOR);
    }

    #[test]
    fn cloud_vertices_skip_alpha_zero_environment() {
        assert!(cloud_vertices(CloudEnvironment::disabled()).is_empty());
        assert!(cloud_vertices(CloudEnvironment::with_color_and_height(
            [1.0, 1.0, 1.0, 0.0],
            VANILLA_DEFAULT_CLOUD_HEIGHT,
        ))
        .is_empty());
    }

    #[test]
    fn cloud_shader_consumes_vanilla_cloud_fog_end_slot() {
        assert!(CLOUD_SHADER.contains("let cloud_end = camera.fog_visibility_ends.y;"));
        assert!(CLOUD_SHADER.contains(
            "color.a *= 1.0 - linear_fog_value(length(input.fog_position), 0.0, cloud_end);"
        ));
    }
}
