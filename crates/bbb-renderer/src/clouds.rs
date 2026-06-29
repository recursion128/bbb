use anyhow::{anyhow, bail, Result};
use wgpu::util::DeviceExt;

use crate::camera::CameraPose;
use crate::gpu::{DepthTarget, DEPTH_FORMAT, DEPTH_TARGET_USAGE};

pub const VANILLA_DEFAULT_CLOUD_COLOR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
pub const VANILLA_DEFAULT_CLOUD_HEIGHT: f32 = 192.33;

const CLOUD_CELL_SIZE_IN_BLOCKS: f32 = 12.0;
const CLOUD_TICKS_PER_CELL: i64 = 400;
const CLOUD_BLOCKS_PER_TICK: f64 = 0.030000001;
const CLOUD_PRESENTATION_HALF_EXTENT: f32 = 2048.0;
const CLOUD_FANCY_HEIGHT_IN_BLOCKS: f32 = 4.0;
const CLOUD_Z_OFFSET_BLOCKS: f64 = 3.96;
const VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD: u8 = 10;

const CLOUD_BOTTOM_FACE_TINT: f32 = 0.7;
const CLOUD_TOP_FACE_TINT: f32 = 1.0;
const CLOUD_NORTH_SOUTH_FACE_TINT: f32 = 0.8;
const CLOUD_WEST_EAST_FACE_TINT: f32 = 0.9;

const CLOUD_COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0)
var cloud_texture: texture_2d<f32>;

@group(0) @binding(1)
var cloud_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(cloud_texture, cloud_sampler, input.uv);
}
"#;

pub(super) const CLOUD_COMPOSITE_BLEND: wgpu::BlendState =
    wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING;

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

struct CloudInfo {
    offset: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> cloud_info: CloudInfo;

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
        input.position.x + cloud_info.offset.x + camera.camera_position.x,
        input.position.y,
        input.position.z + cloud_info.offset.y + camera.camera_position.z,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudFrame {
    pub camera_position: [f32; 3],
    pub game_time: i64,
    pub partial_tick: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudShape {
    Flat,
    Fancy,
}

impl Default for CloudEnvironment {
    fn default() -> Self {
        Self::disabled()
    }
}

impl Default for CloudFrame {
    fn default() -> Self {
        Self::at_camera_position([0.0, 0.0, 0.0], 0, 0.0)
    }
}

impl Default for CloudShape {
    fn default() -> Self {
        Self::Fancy
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

impl CloudFrame {
    pub fn at_camera_position(
        camera_position: [f32; 3],
        game_time: i64,
        partial_tick: f32,
    ) -> Self {
        Self {
            camera_position,
            game_time,
            partial_tick,
        }
        .sanitized()
    }

    pub fn from_camera_pose(pose: CameraPose, game_time: i64, partial_tick: f32) -> Self {
        Self::at_camera_position(
            [
                pose.position[0],
                pose.position[1] + pose.eye_height,
                pose.position[2],
            ],
            game_time,
            partial_tick,
        )
    }

    pub fn sanitized(self) -> Self {
        Self {
            camera_position: [
                sanitize_position(self.camera_position[0]),
                sanitize_position(self.camera_position[1]),
                sanitize_position(self.camera_position[2]),
            ],
            game_time: self.game_time,
            partial_tick: sanitize_unit(self.partial_tick),
        }
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct CloudUniform {
    offset: [f32; 4],
}

#[derive(Debug)]
pub(super) struct CloudTextureData {
    width: usize,
    height: usize,
    non_empty_cells: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CloudMeshKey {
    center_cell_x: i32,
    center_cell_z: i32,
    radius_cells: i32,
    shape: CloudShape,
    relative_camera_pos: Option<CloudRelativeCameraPos>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CloudPlacement {
    mesh_key: CloudMeshKey,
    offset: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloudRelativeCameraPos {
    AboveClouds,
    InsideClouds,
    BelowClouds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloudFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

pub(super) struct CloudGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
    pub(super) mesh_key: Option<CloudMeshKey>,
}

pub(super) struct CloudTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) _sampler: wgpu::Sampler,
    pub(super) depth: DepthTarget,
    pub(super) bind_group: wgpu::BindGroup,
}

pub(super) fn create_cloud_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    cloud_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-cloud-shader"),
        source: wgpu::ShaderSource::Wgsl(CLOUD_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-cloud-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout, cloud_bind_group_layout],
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
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

pub(super) fn create_cloud_target_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-cloud-target-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub(super) fn create_cloud_target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> CloudTarget {
    let texture = create_cloud_target_texture(device, format, width, height);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-cloud-target-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let depth = create_cloud_depth_target(device, width, height);
    let bind_group = create_cloud_target_bind_group(device, layout, &view, &sampler);

    CloudTarget {
        _texture: texture,
        view,
        _sampler: sampler,
        depth,
        bind_group,
    }
}

pub(super) fn create_cloud_composite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-cloud-composite-shader"),
        source: wgpu::ShaderSource::Wgsl(CLOUD_COMPOSITE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-cloud-composite-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-cloud-composite-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(CLOUD_COMPOSITE_BLEND),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_cloud_target_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-cloud-target-color"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn create_cloud_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-cloud-target-depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: DEPTH_TARGET_USAGE,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    DepthTarget {
        _texture: texture,
        view,
    }
}

fn create_cloud_target_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-cloud-target-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

pub(super) fn create_cloud_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-cloud-bind-group-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub(super) fn create_cloud_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-cloud-uniform-buffer"),
        contents: bytemuck::bytes_of(&CloudUniform::default()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

pub(super) fn create_cloud_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-cloud-bind-group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    })
}

pub(super) fn write_cloud_uniform(
    queue: &wgpu::Queue,
    uniform_buffer: &wgpu::Buffer,
    frame: CloudFrame,
    texture: Option<&CloudTextureData>,
) {
    queue.write_buffer(
        uniform_buffer,
        0,
        bytemuck::bytes_of(&cloud_uniform(frame, texture)),
    );
}

pub(super) fn create_cloud_gpu(
    device: &wgpu::Device,
    environment: CloudEnvironment,
    texture: Option<&CloudTextureData>,
    frame: CloudFrame,
    shape: CloudShape,
) -> Option<CloudGpu> {
    let vertices = cloud_vertices(environment, texture, frame, shape);
    if vertices.is_empty() {
        return None;
    }
    let mesh_key = cloud_mesh_key(environment, texture, frame, shape);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-cloud-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    Some(CloudGpu {
        vertex_buffer,
        vertex_count: vertices.len() as u32,
        mesh_key,
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

pub(super) fn cloud_mesh_key(
    environment: CloudEnvironment,
    texture: Option<&CloudTextureData>,
    frame: CloudFrame,
    shape: CloudShape,
) -> Option<CloudMeshKey> {
    texture.map(|texture| cloud_placement(environment, texture, frame, shape).mesh_key)
}

fn cloud_vertices(
    environment: CloudEnvironment,
    texture: Option<&CloudTextureData>,
    frame: CloudFrame,
    shape: CloudShape,
) -> Vec<CloudVertex> {
    let environment = environment.sanitized();
    if !environment.is_visible() {
        return Vec::new();
    }

    if let Some(texture) = texture {
        let placement = cloud_placement(environment, texture, frame, shape);
        return match shape {
            CloudShape::Flat => flat_cloud_cell_vertices(environment, texture, placement.mesh_key),
            CloudShape::Fancy => {
                fancy_cloud_cell_vertices(environment, texture, placement.mesh_key)
            }
        };
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
    mesh_key: CloudMeshKey,
) -> Vec<CloudVertex> {
    let radius_cells = mesh_key.radius_cells.max(0);
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
                        mesh_key,
                        relative_cell_x,
                        -relative_cell_z,
                    );
                }
                append_flat_cloud_cell_if_non_empty(
                    &mut vertices,
                    environment,
                    texture,
                    mesh_key,
                    relative_cell_x,
                    relative_cell_z,
                );
            }
        }
    }
    vertices
}

fn fancy_cloud_cell_vertices(
    environment: CloudEnvironment,
    texture: &CloudTextureData,
    mesh_key: CloudMeshKey,
) -> Vec<CloudVertex> {
    let radius_cells = mesh_key.radius_cells.max(0);
    let relative_camera_pos = mesh_key
        .relative_camera_pos
        .unwrap_or(CloudRelativeCameraPos::InsideClouds);
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
                    append_fancy_cloud_cell_if_non_empty(
                        &mut vertices,
                        environment,
                        texture,
                        mesh_key,
                        relative_camera_pos,
                        relative_cell_x,
                        -relative_cell_z,
                    );
                }
                append_fancy_cloud_cell_if_non_empty(
                    &mut vertices,
                    environment,
                    texture,
                    mesh_key,
                    relative_camera_pos,
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
    mesh_key: CloudMeshKey,
    relative_cell_x: i32,
    relative_cell_z: i32,
) {
    if !texture.is_non_empty(
        mesh_key.center_cell_x,
        mesh_key.center_cell_z,
        relative_cell_x,
        relative_cell_z,
    ) {
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

fn append_fancy_cloud_cell_if_non_empty(
    vertices: &mut Vec<CloudVertex>,
    environment: CloudEnvironment,
    texture: &CloudTextureData,
    mesh_key: CloudMeshKey,
    relative_camera_pos: CloudRelativeCameraPos,
    relative_cell_x: i32,
    relative_cell_z: i32,
) {
    if !texture.is_non_empty(
        mesh_key.center_cell_x,
        mesh_key.center_cell_z,
        relative_cell_x,
        relative_cell_z,
    ) {
        return;
    }

    if relative_camera_pos != CloudRelativeCameraPos::BelowClouds {
        append_cloud_face(
            vertices,
            environment,
            relative_cell_x,
            relative_cell_z,
            CloudFace::Up,
            false,
        );
    }
    if relative_camera_pos != CloudRelativeCameraPos::AboveClouds {
        append_cloud_face(
            vertices,
            environment,
            relative_cell_x,
            relative_cell_z,
            CloudFace::Down,
            false,
        );
    }
    if texture.is_neighbor_empty(
        mesh_key.center_cell_x,
        mesh_key.center_cell_z,
        relative_cell_x,
        relative_cell_z,
        0,
        -1,
    ) && relative_cell_z > 0
    {
        append_cloud_face(
            vertices,
            environment,
            relative_cell_x,
            relative_cell_z,
            CloudFace::North,
            false,
        );
    }
    if texture.is_neighbor_empty(
        mesh_key.center_cell_x,
        mesh_key.center_cell_z,
        relative_cell_x,
        relative_cell_z,
        0,
        1,
    ) && relative_cell_z < 0
    {
        append_cloud_face(
            vertices,
            environment,
            relative_cell_x,
            relative_cell_z,
            CloudFace::South,
            false,
        );
    }
    if texture.is_neighbor_empty(
        mesh_key.center_cell_x,
        mesh_key.center_cell_z,
        relative_cell_x,
        relative_cell_z,
        -1,
        0,
    ) && relative_cell_x > 0
    {
        append_cloud_face(
            vertices,
            environment,
            relative_cell_x,
            relative_cell_z,
            CloudFace::West,
            false,
        );
    }
    if texture.is_neighbor_empty(
        mesh_key.center_cell_x,
        mesh_key.center_cell_z,
        relative_cell_x,
        relative_cell_z,
        1,
        0,
    ) && relative_cell_x < 0
    {
        append_cloud_face(
            vertices,
            environment,
            relative_cell_x,
            relative_cell_z,
            CloudFace::East,
            false,
        );
    }

    if relative_cell_x.abs() <= 1 && relative_cell_z.abs() <= 1 {
        for face in CloudFace::ALL {
            append_cloud_face(
                vertices,
                environment,
                relative_cell_x,
                relative_cell_z,
                face,
                true,
            );
        }
    }
}

fn append_cloud_face(
    vertices: &mut Vec<CloudVertex>,
    environment: CloudEnvironment,
    relative_cell_x: i32,
    relative_cell_z: i32,
    face: CloudFace,
    inside_face: bool,
) {
    let mut quad = cloud_face_quad(environment, relative_cell_x, relative_cell_z, face);
    if inside_face {
        quad.reverse();
    }
    vertices.extend([quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
}

fn cloud_face_quad(
    environment: CloudEnvironment,
    relative_cell_x: i32,
    relative_cell_z: i32,
    face: CloudFace,
) -> [CloudVertex; 4] {
    let x0 = relative_cell_x as f32 * CLOUD_CELL_SIZE_IN_BLOCKS;
    let z0 = relative_cell_z as f32 * CLOUD_CELL_SIZE_IN_BLOCKS;
    let x1 = x0 + CLOUD_CELL_SIZE_IN_BLOCKS;
    let z1 = z0 + CLOUD_CELL_SIZE_IN_BLOCKS;
    let y0 = environment.height;
    let y1 = y0 + CLOUD_FANCY_HEIGHT_IN_BLOCKS;
    let color = cloud_face_color(environment, face);
    let positions = match face {
        CloudFace::Down => [[x1, y0, z0], [x1, y0, z1], [x0, y0, z1], [x0, y0, z0]],
        CloudFace::Up => [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
        CloudFace::North => [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
        CloudFace::South => [[x1, y0, z1], [x1, y1, z1], [x0, y1, z1], [x0, y0, z1]],
        CloudFace::West => [[x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [x0, y0, z0]],
        CloudFace::East => [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
    };
    positions.map(|position| CloudVertex { position, color })
}

fn cloud_face_color(environment: CloudEnvironment, face: CloudFace) -> [f32; 4] {
    let tint = match face {
        CloudFace::Down => CLOUD_BOTTOM_FACE_TINT,
        CloudFace::Up => CLOUD_TOP_FACE_TINT,
        CloudFace::North | CloudFace::South => CLOUD_NORTH_SOUTH_FACE_TINT,
        CloudFace::West | CloudFace::East => CLOUD_WEST_EAST_FACE_TINT,
    };
    [
        environment.color[0] * tint,
        environment.color[1] * tint,
        environment.color[2] * tint,
        environment.color[3],
    ]
}

impl CloudTextureData {
    fn is_non_empty(
        &self,
        center_cell_x: i32,
        center_cell_z: i32,
        relative_cell_x: i32,
        relative_cell_z: i32,
    ) -> bool {
        let x = (center_cell_x + relative_cell_x).rem_euclid(self.width as i32) as usize;
        let z = (center_cell_z + relative_cell_z).rem_euclid(self.height as i32) as usize;
        self.non_empty_cells[x + z * self.width]
    }

    fn is_neighbor_empty(
        &self,
        center_cell_x: i32,
        center_cell_z: i32,
        relative_cell_x: i32,
        relative_cell_z: i32,
        neighbor_x: i32,
        neighbor_z: i32,
    ) -> bool {
        !self.is_non_empty(
            center_cell_x,
            center_cell_z,
            relative_cell_x + neighbor_x,
            relative_cell_z + neighbor_z,
        )
    }
}

impl CloudFace {
    const ALL: [CloudFace; 6] = [
        CloudFace::Down,
        CloudFace::Up,
        CloudFace::North,
        CloudFace::South,
        CloudFace::West,
        CloudFace::East,
    ];
}

impl Default for CloudUniform {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

fn vanilla_cloud_radius_cells(radius_blocks: f32) -> i32 {
    (radius_blocks / CLOUD_CELL_SIZE_IN_BLOCKS).ceil() as i32
}

fn cloud_uniform(frame: CloudFrame, texture: Option<&CloudTextureData>) -> CloudUniform {
    let offset = texture
        .map(|texture| {
            cloud_placement(
                CloudEnvironment::overworld_default(),
                texture,
                frame,
                CloudShape::Flat,
            )
            .offset
        })
        .unwrap_or([0.0, 0.0]);
    CloudUniform {
        offset: [offset[0], offset[1], 0.0, 0.0],
    }
}

fn cloud_placement(
    environment: CloudEnvironment,
    texture: &CloudTextureData,
    frame: CloudFrame,
    shape: CloudShape,
) -> CloudPlacement {
    let environment = environment.sanitized();
    let frame = frame.sanitized();
    let texture_width_blocks = texture.width as f64 * CLOUD_CELL_SIZE_IN_BLOCKS as f64;
    let texture_height_blocks = texture.height as f64 * CLOUD_CELL_SIZE_IN_BLOCKS as f64;
    let period_ticks = texture.width as i64 * CLOUD_TICKS_PER_CELL;
    let cloud_offset_ticks =
        frame.game_time.rem_euclid(period_ticks) as f64 + f64::from(frame.partial_tick);
    let cloud_x = wrap_cloud_coordinate(
        f64::from(frame.camera_position[0]) + cloud_offset_ticks * CLOUD_BLOCKS_PER_TICK,
        texture_width_blocks,
    );
    let cloud_z = wrap_cloud_coordinate(
        f64::from(frame.camera_position[2]) + CLOUD_Z_OFFSET_BLOCKS,
        texture_height_blocks,
    );
    let center_cell_x = (cloud_x / CLOUD_CELL_SIZE_IN_BLOCKS as f64).floor() as i32;
    let center_cell_z = (cloud_z / CLOUD_CELL_SIZE_IN_BLOCKS as f64).floor() as i32;
    let x_in_cell = cloud_x - f64::from(center_cell_x) * CLOUD_CELL_SIZE_IN_BLOCKS as f64;
    let z_in_cell = cloud_z - f64::from(center_cell_z) * CLOUD_CELL_SIZE_IN_BLOCKS as f64;
    CloudPlacement {
        mesh_key: CloudMeshKey {
            center_cell_x,
            center_cell_z,
            radius_cells: vanilla_cloud_radius_cells(CLOUD_PRESENTATION_HALF_EXTENT),
            shape,
            relative_camera_pos: match shape {
                CloudShape::Flat => None,
                CloudShape::Fancy => Some(cloud_relative_camera_pos(environment, frame)),
            },
        },
        offset: [-x_in_cell as f32, -z_in_cell as f32],
    }
}

fn cloud_relative_camera_pos(
    environment: CloudEnvironment,
    frame: CloudFrame,
) -> CloudRelativeCameraPos {
    let relative_bottom_y = environment.height - frame.camera_position[1];
    let relative_top_y = relative_bottom_y + CLOUD_FANCY_HEIGHT_IN_BLOCKS;
    if relative_top_y < 0.0 {
        CloudRelativeCameraPos::AboveClouds
    } else if relative_bottom_y > 0.0 {
        CloudRelativeCameraPos::BelowClouds
    } else {
        CloudRelativeCameraPos::InsideClouds
    }
}

fn wrap_cloud_coordinate(value: f64, period: f64) -> f64 {
    value - (value / period).floor() * period
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

fn sanitize_position(value: f32) -> f32 {
    if value.is_finite() {
        value
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
    use crate::camera::CameraPose;

    use super::{
        basic_cloud_plane_vertices, cloud_placement, cloud_uniform, cloud_vertices,
        create_cloud_texture_data, fancy_cloud_cell_vertices, flat_cloud_cell_vertices,
        vanilla_cloud_radius_cells, CloudEnvironment, CloudFrame, CloudMeshKey,
        CloudRelativeCameraPos, CloudShape, CloudTextureImage, CLOUD_BLOCKS_PER_TICK,
        CLOUD_BOTTOM_FACE_TINT, CLOUD_CELL_SIZE_IN_BLOCKS, CLOUD_COMPOSITE_BLEND,
        CLOUD_COMPOSITE_SHADER, CLOUD_FANCY_HEIGHT_IN_BLOCKS, CLOUD_NORTH_SOUTH_FACE_TINT,
        CLOUD_PRESENTATION_HALF_EXTENT, CLOUD_SHADER, CLOUD_TOP_FACE_TINT, CLOUD_Z_OFFSET_BLOCKS,
        DEPTH_TARGET_USAGE, VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD, VANILLA_DEFAULT_CLOUD_COLOR,
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
    fn cloud_shape_defaults_to_vanilla_fancy() {
        assert_eq!(CloudShape::default(), CloudShape::Fancy);
    }

    #[test]
    fn cloud_pipeline_depth_state_matches_vanilla_default() {
        // Vanilla `RenderPipelines.CLOUDS_SNIPPET` uses `DepthStencilState.DEFAULT`:
        // LEQUAL depth test with depth writes enabled.
        let source = include_str!("clouds.rs");

        assert!(source.contains("depth_write_enabled: true"));
        assert!(source.contains("depth_compare: wgpu::CompareFunction::LessEqual"));
    }

    #[test]
    fn cloud_depth_target_is_bindable_for_transparency_clouds_depth_input() {
        let source = include_str!("clouds.rs");

        assert!(DEPTH_TARGET_USAGE.contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
        assert!(DEPTH_TARGET_USAGE.contains(wgpu::TextureUsages::TEXTURE_BINDING));
        assert!(
            source.contains("usage: DEPTH_TARGET_USAGE"),
            "cloud target depth must be sampleable by the future CloudsDepth transparency input"
        );
    }

    #[test]
    fn cloud_target_composite_uses_vanilla_premultiplied_layer_blend() {
        // `post/transparency.fsh` blends sorted layers as `dst * (1 - src.a) + src.rgb`;
        // clouds target RGB has already been written through the translucent cloud pass.
        assert_eq!(
            CLOUD_COMPOSITE_BLEND.color.src_factor,
            wgpu::BlendFactor::One
        );
        assert_eq!(
            CLOUD_COMPOSITE_BLEND.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(
            CLOUD_COMPOSITE_BLEND.alpha.src_factor,
            wgpu::BlendFactor::One
        );
        assert_eq!(
            CLOUD_COMPOSITE_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert!(
            CLOUD_COMPOSITE_SHADER.contains("return textureSample(cloud_texture"),
            "cloud composite must sample target color without multiplying alpha a second time"
        );
    }

    #[test]
    fn cloud_vertices_build_camera_centered_vanilla_height_plane() {
        let vertices = cloud_vertices(
            CloudEnvironment::overworld_default(),
            None,
            CloudFrame::default(),
            CloudShape::Flat,
        );

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
        assert!(cloud_vertices(
            CloudEnvironment::disabled(),
            None,
            CloudFrame::default(),
            CloudShape::Flat
        )
        .is_empty());
        assert!(cloud_vertices(
            CloudEnvironment::with_color_and_height(
                [1.0, 1.0, 1.0, 0.0],
                VANILLA_DEFAULT_CLOUD_HEIGHT,
            ),
            None,
            CloudFrame::default(),
            CloudShape::Flat
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

        assert!(!data.is_non_empty(0, 0, 0, 0));
        assert!(data.is_non_empty(0, 0, 1, 0));
        assert!(data.is_non_empty(0, 0, -1, 0));
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
        let vertices = flat_cloud_cell_vertices(
            CloudEnvironment::overworld_default(),
            &texture,
            CloudMeshKey {
                center_cell_x: 0,
                center_cell_z: 0,
                radius_cells: 1,
                shape: CloudShape::Flat,
                relative_camera_pos: None,
            },
        );

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
            cloud_vertices(
                CloudEnvironment::overworld_default(),
                None,
                CloudFrame::default(),
                CloudShape::Flat
            ),
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

        assert!(cloud_vertices(
            CloudEnvironment::overworld_default(),
            Some(&texture),
            CloudFrame::default(),
            CloudShape::Flat
        )
        .is_empty());
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
            Some(&texture),
            CloudFrame::default(),
            CloudShape::Flat
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
        let vertices = flat_cloud_cell_vertices(
            environment,
            &texture,
            CloudMeshKey {
                center_cell_x: 0,
                center_cell_z: 0,
                radius_cells: 0,
                shape: CloudShape::Flat,
                relative_camera_pos: None,
            },
        );

        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].color, environment.color);
    }

    #[test]
    fn fancy_cloud_cells_use_vanilla_top_bottom_camera_gates() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 255],
        })
        .unwrap();
        let below_key = CloudMeshKey {
            center_cell_x: 0,
            center_cell_z: 0,
            radius_cells: 0,
            shape: CloudShape::Fancy,
            relative_camera_pos: Some(CloudRelativeCameraPos::BelowClouds),
        };
        let above_key = CloudMeshKey {
            relative_camera_pos: Some(CloudRelativeCameraPos::AboveClouds),
            ..below_key
        };
        let inside_key = CloudMeshKey {
            relative_camera_pos: Some(CloudRelativeCameraPos::InsideClouds),
            ..below_key
        };

        let below =
            fancy_cloud_cell_vertices(CloudEnvironment::overworld_default(), &texture, below_key);
        let above =
            fancy_cloud_cell_vertices(CloudEnvironment::overworld_default(), &texture, above_key);
        let inside =
            fancy_cloud_cell_vertices(CloudEnvironment::overworld_default(), &texture, inside_key);

        assert_eq!(below.len(), 7 * 6);
        assert_eq!(above.len(), 7 * 6);
        assert_eq!(inside.len(), 8 * 6);
        assert_eq!(below[0].color, tinted_cloud_color(CLOUD_BOTTOM_FACE_TINT));
        assert_eq!(above[0].color, tinted_cloud_color(CLOUD_TOP_FACE_TINT));
        assert_eq!(
            above[0].position[1],
            VANILLA_DEFAULT_CLOUD_HEIGHT + CLOUD_FANCY_HEIGHT_IN_BLOCKS
        );
    }

    #[test]
    fn fancy_cloud_cells_add_vanilla_exterior_side_faces_for_empty_neighbors() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 1,
            height: 5,
            rgba: vec![
                0, 0, 0, 0, //
                0, 0, 0, 0, //
                0, 0, 0, 255, //
                0, 0, 0, 0, //
                0, 0, 0, 0,
            ],
        })
        .unwrap();
        let vertices = fancy_cloud_cell_vertices(
            CloudEnvironment::overworld_default(),
            &texture,
            CloudMeshKey {
                center_cell_x: 0,
                center_cell_z: 0,
                radius_cells: 2,
                shape: CloudShape::Fancy,
                relative_camera_pos: Some(CloudRelativeCameraPos::InsideClouds),
            },
        );

        assert_eq!(vertices.len(), 3 * 6);
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.color == tinted_cloud_color(CLOUD_NORTH_SOUTH_FACE_TINT)),
            "north side face uses vanilla side tint when the neighboring cloud cell is empty"
        );
    }

    #[test]
    fn cloud_frame_from_camera_pose_uses_eye_position() {
        let frame = CloudFrame::from_camera_pose(
            CameraPose {
                position: [1.0, 64.0, 3.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            },
            42,
            0.25,
        );

        assert_eq!(frame.camera_position, [1.0, 65.62, 3.0]);
        assert_eq!(frame.game_time, 42);
        assert_eq!(frame.partial_tick, 0.25);
    }

    #[test]
    fn cloud_placement_uses_vanilla_camera_cell_offset() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 3,
            height: 1,
            rgba: vec![
                0, 0, 0, 0, //
                0, 0, 0, 255, //
                0, 0, 0, 0,
            ],
        })
        .unwrap();
        let frame = CloudFrame::at_camera_position([12.5, 70.0, -5.0], 0, 0.0);
        let placement = cloud_placement(
            CloudEnvironment::overworld_default(),
            &texture,
            frame,
            CloudShape::Flat,
        );

        assert_eq!(placement.mesh_key.center_cell_x, 1);
        assert_eq!(placement.mesh_key.center_cell_z, 0);
        assert_close_f32(placement.offset[0], -0.5);
        assert_close_f32(
            placement.offset[1],
            -((12.0 + (-5.0 + CLOUD_Z_OFFSET_BLOCKS) as f32) % 12.0),
        );
        assert_eq!(
            cloud_uniform(frame, Some(&texture)).offset,
            [placement.offset[0], placement.offset[1], 0.0, 0.0]
        );

        let vertices = flat_cloud_cell_vertices(
            CloudEnvironment::overworld_default(),
            &texture,
            CloudMeshKey {
                radius_cells: 0,
                ..placement.mesh_key
            },
        );
        assert_eq!(vertices.len(), 6);
    }

    #[test]
    fn cloud_placement_moves_x_with_vanilla_game_time_speed() {
        let texture = create_cloud_texture_data(&CloudTextureImage {
            width: 4,
            height: 1,
            rgba: vec![
                0, 0, 0, 255, //
                0, 0, 0, 255, //
                0, 0, 0, 255, //
                0, 0, 0, 255,
            ],
        })
        .unwrap();
        let frame = CloudFrame::at_camera_position([0.0, 70.0, 0.0], 399, 0.5);
        let placement = cloud_placement(
            CloudEnvironment::overworld_default(),
            &texture,
            frame,
            CloudShape::Flat,
        );
        let expected_x_in_cell = (399.5 * CLOUD_BLOCKS_PER_TICK) as f32;

        assert_eq!(placement.mesh_key.center_cell_x, 0);
        assert_close_f32(placement.offset[0], -expected_x_in_cell);

        let next_cell = cloud_placement(
            CloudEnvironment::overworld_default(),
            &texture,
            CloudFrame::at_camera_position([0.0, 70.0, 0.0], 401, 0.0),
            CloudShape::Flat,
        );
        assert_eq!(next_cell.mesh_key.center_cell_x, 1);
    }

    #[test]
    fn cloud_shader_consumes_vanilla_cloud_fog_end_slot() {
        assert!(CLOUD_SHADER.contains("let cloud_end = camera.fog_visibility_ends.y;"));
        assert!(CLOUD_SHADER.contains("@group(1) @binding(0)"));
        assert!(CLOUD_SHADER.contains("input.position.x + cloud_info.offset.x"));
        assert!(CLOUD_SHADER.contains("input.position.z + cloud_info.offset.y"));
        assert!(CLOUD_SHADER.contains(
            "color.a *= 1.0 - linear_fog_value(length(input.fog_position), 0.0, cloud_end);"
        ));
    }

    fn assert_close_f32(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "actual {actual} expected {expected}"
        );
    }

    fn tinted_cloud_color(tint: f32) -> [f32; 4] {
        [
            VANILLA_DEFAULT_CLOUD_COLOR[0] * tint,
            VANILLA_DEFAULT_CLOUD_COLOR[1] * tint,
            VANILLA_DEFAULT_CLOUD_COLOR[2] * tint,
            VANILLA_DEFAULT_CLOUD_COLOR[3],
        ]
    }
}
