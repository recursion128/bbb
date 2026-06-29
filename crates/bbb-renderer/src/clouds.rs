use anyhow::{anyhow, bail, Result};
use wgpu::util::DeviceExt;

use crate::gpu::DEPTH_FORMAT;

pub const VANILLA_DEFAULT_CLOUD_COLOR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
pub const VANILLA_DEFAULT_CLOUD_HEIGHT: f32 = 192.33;

const CLOUD_CELL_SIZE_IN_BLOCKS: f32 = 12.0;
const CLOUD_PRESENTATION_HALF_EXTENT: f32 = 2048.0;
const VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD: u8 = 10;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudTextureImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct CloudVertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[derive(Debug)]
pub(super) struct CloudTextureData {
    width: usize,
    height: usize,
    non_empty_cells: Vec<bool>,
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
    texture: Option<&CloudTextureData>,
) -> Option<CloudGpu> {
    let vertices = cloud_vertices(environment, texture);
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

pub(super) fn create_cloud_texture_data(image: &CloudTextureImage) -> Result<CloudTextureData> {
    validate_cloud_texture_image(image)?;
    let cell_count = (image.width as usize)
        .checked_mul(image.height as usize)
        .ok_or_else(|| anyhow!("cloud texture cell count overflow"))?;
    let mut non_empty_cells = Vec::with_capacity(cell_count);
    for pixel in image.rgba.chunks_exact(4) {
        non_empty_cells.push(pixel[3] >= VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD);
    }
    Ok(CloudTextureData {
        width: image.width as usize,
        height: image.height as usize,
        non_empty_cells,
    })
}

fn cloud_vertices(
    environment: CloudEnvironment,
    texture: Option<&CloudTextureData>,
) -> Vec<CloudVertex> {
    let environment = environment.sanitized();
    if !environment.is_visible() {
        return Vec::new();
    }

    if let Some(texture) = texture {
        return flat_cloud_cell_vertices(
            environment,
            texture,
            vanilla_cloud_radius_cells(CLOUD_PRESENTATION_HALF_EXTENT),
        );
    }

    basic_cloud_plane_vertices(environment)
}

fn basic_cloud_plane_vertices(environment: CloudEnvironment) -> Vec<CloudVertex> {
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

fn flat_cloud_cell_vertices(
    environment: CloudEnvironment,
    texture: &CloudTextureData,
    radius_cells: i32,
) -> Vec<CloudVertex> {
    let radius_cells = radius_cells.max(0);
    let mut vertices = Vec::new();
    for ring in 0..=2 * radius_cells {
        for relative_cell_x in -ring..=ring {
            let relative_cell_z = ring - relative_cell_x.abs();
            if relative_cell_z >= 0
                && relative_cell_z <= radius_cells
                && relative_cell_x * relative_cell_x + relative_cell_z * relative_cell_z
                    <= radius_cells * radius_cells
            {
                if relative_cell_z != 0 {
                    append_flat_cloud_cell_if_non_empty(
                        &mut vertices,
                        environment,
                        texture,
                        relative_cell_x,
                        -relative_cell_z,
                    );
                }
                append_flat_cloud_cell_if_non_empty(
                    &mut vertices,
                    environment,
                    texture,
                    relative_cell_x,
                    relative_cell_z,
                );
            }
        }
    }
    vertices
}

fn append_flat_cloud_cell_if_non_empty(
    vertices: &mut Vec<CloudVertex>,
    environment: CloudEnvironment,
    texture: &CloudTextureData,
    relative_cell_x: i32,
    relative_cell_z: i32,
) {
    if !texture.is_non_empty(relative_cell_x, relative_cell_z) {
        return;
    }
    let x0 = relative_cell_x as f32 * CLOUD_CELL_SIZE_IN_BLOCKS;
    let z0 = relative_cell_z as f32 * CLOUD_CELL_SIZE_IN_BLOCKS;
    let x1 = x0 + CLOUD_CELL_SIZE_IN_BLOCKS;
    let z1 = z0 + CLOUD_CELL_SIZE_IN_BLOCKS;
    let y = environment.height;
    let quad = [
        CloudVertex {
            position: [x1, y, z0],
            color: environment.color,
        },
        CloudVertex {
            position: [x1, y, z1],
            color: environment.color,
        },
        CloudVertex {
            position: [x0, y, z1],
            color: environment.color,
        },
        CloudVertex {
            position: [x0, y, z0],
            color: environment.color,
        },
    ];
    vertices.extend([quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
}

impl CloudTextureData {
    fn is_non_empty(&self, cell_x: i32, cell_z: i32) -> bool {
        let x = cell_x.rem_euclid(self.width as i32) as usize;
        let z = cell_z.rem_euclid(self.height as i32) as usize;
        self.non_empty_cells[x + z * self.width]
    }
}

fn vanilla_cloud_radius_cells(radius_blocks: f32) -> i32 {
    (radius_blocks / CLOUD_CELL_SIZE_IN_BLOCKS).ceil() as i32
}

fn validate_cloud_texture_image(image: &CloudTextureImage) -> Result<()> {
    if image.width == 0 || image.height == 0 {
        bail!(
            "cloud texture dimensions must be non-zero, got {}x{}",
            image.width,
            image.height
        );
    }
    let expected = (image.width as usize)
        .checked_mul(image.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("cloud texture byte count overflow"))?;
    if image.rgba.len() != expected {
        bail!(
            "cloud texture has {} RGBA bytes, expected {} for {}x{}",
            image.rgba.len(),
            expected,
            image.width,
            image.height
        );
    }
    Ok(())
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
        basic_cloud_plane_vertices, cloud_vertices, create_cloud_texture_data,
        flat_cloud_cell_vertices, vanilla_cloud_radius_cells, CloudEnvironment, CloudTextureImage,
        CLOUD_CELL_SIZE_IN_BLOCKS, CLOUD_PRESENTATION_HALF_EXTENT, CLOUD_SHADER,
        VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD, VANILLA_DEFAULT_CLOUD_COLOR,
        VANILLA_DEFAULT_CLOUD_HEIGHT,
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
        let vertices = cloud_vertices(CloudEnvironment::overworld_default(), None);

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
        assert!(cloud_vertices(CloudEnvironment::disabled(), None).is_empty());
        assert!(cloud_vertices(
            CloudEnvironment::with_color_and_height(
                [1.0, 1.0, 1.0, 0.0],
                VANILLA_DEFAULT_CLOUD_HEIGHT,
            ),
            None
        )
        .is_empty());
    }

    #[test]
    fn cloud_texture_data_uses_vanilla_alpha_empty_threshold() {
        let data = create_cloud_texture_data(&CloudTextureImage {
            width: 2,
            height: 1,
            rgba: vec![
                0,
                0,
                0,
                VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD - 1,
                0,
                0,
                0,
                VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD,
            ],
        })
        .unwrap();

        assert!(!data.is_non_empty(0, 0));
        assert!(data.is_non_empty(1, 0));
        assert!(data.is_non_empty(-1, 0));
    }

    #[test]
    fn flat_cloud_cell_vertices_use_uploaded_cloud_texture_cells() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 2,
            height: 2,
            rgba: vec![
                0, 0, 0, 255, 0, 0, 0, 0, //
                0, 0, 0, 0, 0, 0, 0, 0,
            ],
        })
        .unwrap();
        let vertices = flat_cloud_cell_vertices(CloudEnvironment::overworld_default(), &texture, 1);

        assert_eq!(vertices.len(), 6);
        assert_eq!(
            vertices[0].position,
            [CLOUD_CELL_SIZE_IN_BLOCKS, VANILLA_DEFAULT_CLOUD_HEIGHT, 0.0]
        );
        assert_eq!(
            vertices[2].position,
            [0.0, VANILLA_DEFAULT_CLOUD_HEIGHT, CLOUD_CELL_SIZE_IN_BLOCKS]
        );
        assert_eq!(vertices[0].color, VANILLA_DEFAULT_CLOUD_COLOR);
    }

    #[test]
    fn cloud_vertices_falls_back_to_basic_plane_without_cloud_texture() {
        assert_eq!(
            cloud_vertices(CloudEnvironment::overworld_default(), None),
            basic_cloud_plane_vertices(CloudEnvironment::overworld_default())
        );
    }

    #[test]
    fn vanilla_cloud_radius_matches_cloud_renderer_cell_size() {
        assert_eq!(
            vanilla_cloud_radius_cells(CLOUD_PRESENTATION_HALF_EXTENT),
            (CLOUD_PRESENTATION_HALF_EXTENT / CLOUD_CELL_SIZE_IN_BLOCKS).ceil() as i32
        );
    }

    #[test]
    fn cloud_texture_rejects_invalid_rgba_dimensions() {
        let err = create_cloud_texture_data(&CloudTextureImage {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0],
        })
        .unwrap_err();

        assert!(err.to_string().contains("expected 4 for 1x1"));
        assert!(create_cloud_texture_data(&CloudTextureImage {
            width: 0,
            height: 1,
            rgba: Vec::new(),
        })
        .is_err());
    }

    #[test]
    fn cloud_vertices_skip_when_uploaded_texture_has_no_cells() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 1,
            height: 1,
            rgba: vec![1, 1, 1, VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD - 1],
        })
        .unwrap();

        assert!(cloud_vertices(CloudEnvironment::overworld_default(), Some(&texture)).is_empty());
    }

    #[test]
    fn cloud_vertices_skip_alpha_zero_environment_with_texture() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 255],
        })
        .unwrap();
        assert!(cloud_vertices(
            CloudEnvironment::with_color_and_height(
                [1.0, 1.0, 1.0, 0.0],
                VANILLA_DEFAULT_CLOUD_HEIGHT,
            ),
            Some(&texture)
        )
        .is_empty());
    }

    #[test]
    fn cloud_vertices_keep_basic_plane_extent_constant() {
        let vertices = basic_cloud_plane_vertices(CloudEnvironment::overworld_default());

        assert_eq!(
            vertices[0].position,
            [
                -CLOUD_PRESENTATION_HALF_EXTENT,
                VANILLA_DEFAULT_CLOUD_HEIGHT,
                -CLOUD_PRESENTATION_HALF_EXTENT,
            ]
        );
    }

    #[test]
    fn cloud_vertices_do_not_use_texture_color_for_flat_cells() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
        })
        .unwrap();
        let environment = CloudEnvironment::with_color_and_height(
            [0.25, 0.5, 0.75, 1.0],
            VANILLA_DEFAULT_CLOUD_HEIGHT,
        );
        let vertices = flat_cloud_cell_vertices(environment, &texture, 0);

        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].color, environment.color);
    }

    #[test]
    fn cloud_shader_consumes_vanilla_cloud_fog_end_slot() {
        assert!(CLOUD_SHADER.contains("let cloud_end = camera.fog_visibility_ends.y;"));
        assert!(CLOUD_SHADER.contains(
            "color.a *= 1.0 - linear_fog_value(length(input.fog_position), 0.0, cloud_end);"
        ));
    }
}
