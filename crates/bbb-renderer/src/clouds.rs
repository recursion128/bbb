use anyhow::{anyhow, bail, Result};
use wgpu::util::DeviceExt;

use crate::camera::CameraPose;
use crate::gpu::{DepthTarget, DEPTH_FORMAT, DEPTH_TARGET_USAGE};
use crate::pipeline_builder::{depth_stencil_state, RenderPipelineBuilder};

pub const VANILLA_DEFAULT_CLOUD_COLOR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
pub const VANILLA_DEFAULT_CLOUD_HEIGHT: f32 = 192.33;
pub const VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS: u32 = 128;
pub const VANILLA_MIN_CLOUD_RANGE_CHUNKS: u32 = 2;
pub const VANILLA_MAX_CLOUD_RANGE_CHUNKS: u32 = 128;

const CLOUD_CELL_SIZE_IN_BLOCKS: f32 = 12.0;
const CLOUD_TICKS_PER_CELL: i64 = 400;
const CLOUD_BLOCKS_PER_TICK: f64 = 0.030000001;
#[cfg(test)]
const CLOUD_PRESENTATION_HALF_EXTENT: f32 = VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS as f32 * 16.0;
const CLOUD_FANCY_HEIGHT_IN_BLOCKS: f32 = 4.0;
const CLOUD_Z_OFFSET_BLOCKS: f64 = 3.96;
const VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD: u8 = 10;

#[cfg(test)]
const CLOUD_BOTTOM_FACE_TINT: f32 = 0.7;
#[cfg(test)]
const CLOUD_TOP_FACE_TINT: f32 = 1.0;
#[cfg(test)]
const CLOUD_NORTH_SOUTH_FACE_TINT: f32 = 0.8;
#[cfg(test)]
const CLOUD_WEST_EAST_FACE_TINT: f32 = 0.9;

pub(super) const CLOUD_TARGET_BLEND: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;
const CLOUD_FLAT_PIPELINE_CULL_MODE: Option<wgpu::Face> = None;
const CLOUD_FANCY_PIPELINE_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);
const CLOUD_PIPELINE_DEPTH_WRITE_ENABLED: bool = true;
const CLOUD_PIPELINE_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;
const CLOUD_FLAG_INSIDE_FACE: u8 = 1 << 4;
const CLOUD_FLAG_USE_TOP_COLOR: u8 = 1 << 5;
const CLOUD_FLAG_EXTRA_Z: u8 = 1 << 6;
const CLOUD_FLAG_EXTRA_X: u8 = 1 << 7;
const CLOUD_QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

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
    color: vec4<f32>,
    offset: vec4<f32>,
    cell_size: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> cloud_info: CloudInfo;

struct VertexIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) packed_face: u32,
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

fn signed_byte(value: u32) -> i32 {
    let byte = i32(value & 255u);
    return select(byte, byte - 256, byte >= 128);
}

fn face_vertex(direction: u32, vertex: u32) -> vec3<f32> {
    let half = f32(vertex / 2u);
    let middle = select(0.0, 1.0, vertex == 1u || vertex == 2u);
    switch direction {
        case 0u: { return vec3<f32>(1.0 - half, 0.0, middle); }
        case 1u: { return vec3<f32>(half, 1.0, middle); }
        case 2u: { return vec3<f32>(half, middle, 0.0); }
        case 3u: { return vec3<f32>(1.0 - half, middle, 1.0); }
        case 4u: { return vec3<f32>(0.0, middle, 1.0 - half); }
        default: { return vec3<f32>(1.0, middle, half); }
    }
}

fn face_tint(direction: u32) -> f32 {
    switch direction {
        case 0u: { return 0.7; }
        case 1u: { return 1.0; }
        case 2u, 3u: { return 0.8; }
        default: { return 0.9; }
    }
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let packed = input.packed_face;
    let flags = (packed >> 16u) & 255u;
    let direction = flags & 7u;
    let inside_face = (flags & 16u) != 0u;
    let use_top_color = (flags & 32u) != 0u;
    let cell_x = signed_byte(packed) * 2 + i32((flags >> 7u) & 1u);
    let cell_z = signed_byte(packed >> 8u) * 2 + i32((flags >> 6u) & 1u);
    let vertex = input.vertex_index;
    let directed_vertex = select(vertex, 3u - vertex, inside_face);
    let position_in_cell = face_vertex(direction, directed_vertex);
    let world_position = position_in_cell * cloud_info.cell_size.xyz
        + vec3<f32>(f32(cell_x) * cloud_info.cell_size.x, 0.0,
                    f32(cell_z) * cloud_info.cell_size.z)
        + cloud_info.offset.xyz;
    out.position = camera.view_proj * vec4<f32>(world_position, 1.0);
    let tint = select(face_tint(direction), 1.0, use_top_color);
    out.color = vec4<f32>(cloud_info.color.rgb * tint, cloud_info.color.a);
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
    pub range_chunks: u32,
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
            range_chunks: VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
        }
    }

    pub fn overworld_default() -> Self {
        Self {
            color: VANILLA_DEFAULT_CLOUD_COLOR,
            height: VANILLA_DEFAULT_CLOUD_HEIGHT,
            range_chunks: VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
        }
    }

    pub fn with_color_and_height(color: [f32; 4], height: f32) -> Self {
        Self {
            color,
            height,
            range_chunks: VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
        }
        .sanitized()
    }

    pub fn with_range_chunks(mut self, range_chunks: u32) -> Self {
        self.range_chunks = range_chunks;
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
            height: sanitize_height(self.height),
            range_chunks: self.range_chunks.clamp(
                VANILLA_MIN_CLOUD_RANGE_CHUNKS,
                VANILLA_MAX_CLOUD_RANGE_CHUNKS,
            ),
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
struct CloudFaceRecord {
    packed: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct CloudVertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct CloudUniform {
    color: [f32; 4],
    offset: [f32; 4],
    cell_size: [f32; 4],
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
    pub(super) face_buffer: wgpu::Buffer,
    pub(super) quad_index_buffer: wgpu::Buffer,
    pub(super) face_count: u32,
    pub(super) mesh_key: Option<CloudMeshKey>,
}

pub(super) struct CloudTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) depth: DepthTarget,
}

pub(super) fn create_cloud_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    cloud_bind_group_layout: &wgpu::BindGroupLayout,
    shape: CloudShape,
) -> wgpu::RenderPipeline {
    let (pipeline_label, cull_mode) = match shape {
        CloudShape::Flat => ("bbb-cloud-flat-pipeline", CLOUD_FLAT_PIPELINE_CULL_MODE),
        CloudShape::Fancy => ("bbb-cloud-fancy-pipeline", CLOUD_FANCY_PIPELINE_CULL_MODE),
    };
    RenderPipelineBuilder::new(device, pipeline_label)
        .shader("bbb-cloud-shader", CLOUD_SHADER)
        .layout(
            "bbb-cloud-pipeline-layout",
            &[bind_group_layout, cloud_bind_group_layout],
        )
        .vertex_buffers(&[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CloudFaceRecord>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![0 => Uint32],
        }])
        .color_target(format, Some(CLOUD_TARGET_BLEND))
        .cull_mode(cull_mode)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            CLOUD_PIPELINE_DEPTH_WRITE_ENABLED,
            CLOUD_PIPELINE_DEPTH_COMPARE,
        ))
        .build()
}

pub(super) fn create_cloud_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> CloudTarget {
    let texture = create_cloud_target_texture(device, format, width, height);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = create_cloud_depth_target(device, width, height);

    CloudTarget {
        _texture: texture,
        view,
        depth,
    }
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
    environment: CloudEnvironment,
    frame: CloudFrame,
    texture: Option<&CloudTextureData>,
) {
    queue.write_buffer(
        uniform_buffer,
        0,
        bytemuck::bytes_of(&cloud_uniform(environment, frame, texture)),
    );
}

pub(super) fn create_cloud_gpu(
    device: &wgpu::Device,
    environment: CloudEnvironment,
    texture: Option<&CloudTextureData>,
    frame: CloudFrame,
    shape: CloudShape,
) -> Option<CloudGpu> {
    let faces = cloud_face_records(environment, texture, frame, shape);
    if faces.is_empty() {
        return None;
    }
    let mesh_key = cloud_mesh_key(environment, texture, frame, shape);
    let face_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-cloud-faces"),
        contents: bytemuck::cast_slice(&faces),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-cloud-quad-indices"),
        contents: bytemuck::cast_slice(&CLOUD_QUAD_INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });
    Some(CloudGpu {
        face_buffer,
        quad_index_buffer,
        face_count: faces.len() as u32,
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

fn cloud_face_records(
    environment: CloudEnvironment,
    texture: Option<&CloudTextureData>,
    frame: CloudFrame,
    shape: CloudShape,
) -> Vec<CloudFaceRecord> {
    let environment = environment.sanitized();
    if !environment.is_visible() {
        return Vec::new();
    }

    if let Some(texture) = texture {
        let placement = cloud_placement(environment, texture, frame, shape);
        return match shape {
            CloudShape::Flat => flat_cloud_cell_face_records(texture, placement.mesh_key),
            CloudShape::Fancy => fancy_cloud_cell_face_records(texture, placement.mesh_key),
        };
    }

    vec![encode_cloud_face(0, 0, CloudFace::Down, false, true)]
}

#[cfg(test)]
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
    let Some(texture) = texture else {
        return basic_cloud_plane_vertices(environment);
    };
    let placement = cloud_placement(environment, texture, frame, shape);
    match shape {
        CloudShape::Flat => flat_cloud_cell_vertices(environment, texture, placement.mesh_key),
        CloudShape::Fancy => fancy_cloud_cell_vertices(environment, texture, placement.mesh_key),
    }
}

#[cfg(test)]
fn basic_cloud_plane_vertices(environment: CloudEnvironment) -> Vec<CloudVertex> {
    let y = environment.height;
    let extent = cloud_range_blocks(environment);
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

#[cfg(test)]
fn flat_cloud_cell_vertices(
    environment: CloudEnvironment,
    texture: &CloudTextureData,
    mesh_key: CloudMeshKey,
) -> Vec<CloudVertex> {
    expand_cloud_faces(
        environment,
        &flat_cloud_cell_face_records(texture, mesh_key),
    )
}

fn flat_cloud_cell_face_records(
    texture: &CloudTextureData,
    mesh_key: CloudMeshKey,
) -> Vec<CloudFaceRecord> {
    let radius_cells = mesh_key.radius_cells.max(0);
    let mut faces = Vec::new();
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
                        &mut faces,
                        texture,
                        mesh_key,
                        relative_cell_x,
                        -relative_cell_z,
                    );
                }
                append_flat_cloud_cell_if_non_empty(
                    &mut faces,
                    texture,
                    mesh_key,
                    relative_cell_x,
                    relative_cell_z,
                );
            }
        }
    }
    faces
}

#[cfg(test)]
fn fancy_cloud_cell_vertices(
    environment: CloudEnvironment,
    texture: &CloudTextureData,
    mesh_key: CloudMeshKey,
) -> Vec<CloudVertex> {
    expand_cloud_faces(
        environment,
        &fancy_cloud_cell_face_records(texture, mesh_key),
    )
}

fn fancy_cloud_cell_face_records(
    texture: &CloudTextureData,
    mesh_key: CloudMeshKey,
) -> Vec<CloudFaceRecord> {
    let radius_cells = mesh_key.radius_cells.max(0);
    let relative_camera_pos = mesh_key
        .relative_camera_pos
        .unwrap_or(CloudRelativeCameraPos::InsideClouds);
    let mut faces = Vec::new();
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
                        &mut faces,
                        texture,
                        mesh_key,
                        relative_camera_pos,
                        relative_cell_x,
                        -relative_cell_z,
                    );
                }
                append_fancy_cloud_cell_if_non_empty(
                    &mut faces,
                    texture,
                    mesh_key,
                    relative_camera_pos,
                    relative_cell_x,
                    relative_cell_z,
                );
            }
        }
    }
    faces
}

fn append_flat_cloud_cell_if_non_empty(
    faces: &mut Vec<CloudFaceRecord>,
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
    faces.push(encode_cloud_face(
        relative_cell_x,
        relative_cell_z,
        CloudFace::Down,
        false,
        true,
    ));
}

fn append_fancy_cloud_cell_if_non_empty(
    faces: &mut Vec<CloudFaceRecord>,
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
            faces,
            relative_cell_x,
            relative_cell_z,
            CloudFace::Up,
            false,
        );
    }
    if relative_camera_pos != CloudRelativeCameraPos::AboveClouds {
        append_cloud_face(
            faces,
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
            faces,
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
            faces,
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
            faces,
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
            faces,
            relative_cell_x,
            relative_cell_z,
            CloudFace::East,
            false,
        );
    }

    if relative_cell_x.abs() <= 1 && relative_cell_z.abs() <= 1 {
        for face in CloudFace::ALL {
            append_cloud_face(faces, relative_cell_x, relative_cell_z, face, true);
        }
    }
}

fn append_cloud_face(
    faces: &mut Vec<CloudFaceRecord>,
    relative_cell_x: i32,
    relative_cell_z: i32,
    face: CloudFace,
    inside_face: bool,
) {
    faces.push(encode_cloud_face(
        relative_cell_x,
        relative_cell_z,
        face,
        inside_face,
        false,
    ));
}

fn encode_cloud_face(
    cell_x: i32,
    cell_z: i32,
    face: CloudFace,
    inside_face: bool,
    use_top_color: bool,
) -> CloudFaceRecord {
    debug_assert!((-256..=255).contains(&cell_x));
    debug_assert!((-256..=255).contains(&cell_z));
    let mut flags = face.direction_id();
    if inside_face {
        flags |= CLOUD_FLAG_INSIDE_FACE;
    }
    if use_top_color {
        flags |= CLOUD_FLAG_USE_TOP_COLOR;
    }
    flags |= (cell_x as u8 & 1) * CLOUD_FLAG_EXTRA_X;
    flags |= (cell_z as u8 & 1) * CLOUD_FLAG_EXTRA_Z;
    let cell_x_high = (cell_x >> 1) as i8 as u8;
    let cell_z_high = (cell_z >> 1) as i8 as u8;
    CloudFaceRecord {
        packed: u32::from(cell_x_high) | (u32::from(cell_z_high) << 8) | (u32::from(flags) << 16),
    }
}

#[cfg(test)]
fn expand_cloud_faces(
    environment: CloudEnvironment,
    faces: &[CloudFaceRecord],
) -> Vec<CloudVertex> {
    let mut vertices = Vec::with_capacity(faces.len() * 6);
    for record in faces {
        let cell_x_high = record.packed as u8 as i8 as i32;
        let cell_z_high = (record.packed >> 8) as u8 as i8 as i32;
        let flags = (record.packed >> 16) as u8;
        let cell_x = (cell_x_high << 1) | i32::from(flags & CLOUD_FLAG_EXTRA_X != 0);
        let cell_z = (cell_z_high << 1) | i32::from(flags & CLOUD_FLAG_EXTRA_Z != 0);
        let face = CloudFace::from_direction_id(flags & 7);
        let mut quad = cloud_face_quad(environment, cell_x, cell_z, face);
        if flags & CLOUD_FLAG_USE_TOP_COLOR != 0 {
            for vertex in &mut quad {
                vertex.color = environment.color;
            }
        }
        if flags & CLOUD_FLAG_INSIDE_FACE != 0 {
            quad.reverse();
        }
        vertices.extend([quad[0], quad[1], quad[2], quad[2], quad[3], quad[0]]);
    }
    vertices
}

#[cfg(test)]
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

#[cfg(test)]
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

    fn direction_id(self) -> u8 {
        match self {
            Self::Down => 0,
            Self::Up => 1,
            Self::North => 2,
            Self::South => 3,
            Self::West => 4,
            Self::East => 5,
        }
    }

    #[cfg(test)]
    fn from_direction_id(direction: u8) -> Self {
        match direction {
            0 => Self::Down,
            1 => Self::Up,
            2 => Self::North,
            3 => Self::South,
            4 => Self::West,
            5 => Self::East,
            _ => panic!("invalid cloud direction {direction}"),
        }
    }
}

impl Default for CloudUniform {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 0.0],
            offset: [0.0, 0.0, 0.0, 0.0],
            cell_size: [
                CLOUD_CELL_SIZE_IN_BLOCKS,
                CLOUD_FANCY_HEIGHT_IN_BLOCKS,
                CLOUD_CELL_SIZE_IN_BLOCKS,
                0.0,
            ],
        }
    }
}

fn vanilla_cloud_radius_cells(radius_blocks: f32) -> i32 {
    (radius_blocks / CLOUD_CELL_SIZE_IN_BLOCKS).ceil() as i32
}

fn cloud_uniform(
    environment: CloudEnvironment,
    frame: CloudFrame,
    texture: Option<&CloudTextureData>,
) -> CloudUniform {
    let environment = environment.sanitized();
    let frame = frame.sanitized();
    let (offset, cell_size) = if let Some(texture) = texture {
        let placement = cloud_placement(environment, texture, frame, CloudShape::Flat);
        (
            [
                frame.camera_position[0] + placement.offset[0],
                environment.height,
                frame.camera_position[2] + placement.offset[1],
                0.0,
            ],
            [
                CLOUD_CELL_SIZE_IN_BLOCKS,
                CLOUD_FANCY_HEIGHT_IN_BLOCKS,
                CLOUD_CELL_SIZE_IN_BLOCKS,
                0.0,
            ],
        )
    } else {
        let extent = cloud_range_blocks(environment);
        (
            [
                frame.camera_position[0] - extent,
                environment.height,
                frame.camera_position[2] - extent,
                0.0,
            ],
            [extent * 2.0, 0.0, extent * 2.0, 0.0],
        )
    };
    CloudUniform {
        color: environment.color,
        offset,
        cell_size,
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
            radius_cells: vanilla_cloud_radius_cells(cloud_range_blocks(environment)),
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

fn cloud_range_blocks(environment: CloudEnvironment) -> f32 {
    environment.range_chunks as f32 * 16.0
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
        create_cloud_bind_group_layout, create_cloud_pipeline, create_cloud_texture_data,
        encode_cloud_face, fancy_cloud_cell_vertices, flat_cloud_cell_vertices, CloudEnvironment,
        CloudFace, CloudFaceRecord, CloudFrame, CloudMeshKey, CloudRelativeCameraPos, CloudShape,
        CloudTextureImage, CLOUD_BLOCKS_PER_TICK, CLOUD_BOTTOM_FACE_TINT,
        CLOUD_CELL_SIZE_IN_BLOCKS, CLOUD_FANCY_HEIGHT_IN_BLOCKS, CLOUD_FANCY_PIPELINE_CULL_MODE,
        CLOUD_FLAT_PIPELINE_CULL_MODE, CLOUD_NORTH_SOUTH_FACE_TINT, CLOUD_PIPELINE_DEPTH_COMPARE,
        CLOUD_PIPELINE_DEPTH_WRITE_ENABLED, CLOUD_PRESENTATION_HALF_EXTENT, CLOUD_QUAD_INDICES,
        CLOUD_SHADER, CLOUD_TARGET_BLEND, CLOUD_TOP_FACE_TINT, CLOUD_Z_OFFSET_BLOCKS,
        DEPTH_TARGET_USAGE, VANILLA_CLOUD_EMPTY_ALPHA_THRESHOLD, VANILLA_DEFAULT_CLOUD_COLOR,
        VANILLA_DEFAULT_CLOUD_HEIGHT,
    };

    fn request_test_device() -> Option<wgpu::Device> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("bbb-cloud-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            },
            None,
        ))
        .ok()
        .map(|(device, _queue)| device)
    }

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
    fn cloud_pipeline_state_matches_vanilla_cloud_variants() {
        assert_eq!(CLOUD_FANCY_PIPELINE_CULL_MODE, Some(wgpu::Face::Back));
        assert_eq!(CLOUD_FLAT_PIPELINE_CULL_MODE, None);
        assert!(CLOUD_PIPELINE_DEPTH_WRITE_ENABLED);
        assert_eq!(
            CLOUD_PIPELINE_DEPTH_COMPARE,
            wgpu::CompareFunction::LessEqual
        );
        assert_eq!(
            CLOUD_TARGET_BLEND.color.src_factor,
            wgpu::BlendFactor::SrcAlpha
        );
        assert_eq!(
            CLOUD_TARGET_BLEND.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(CLOUD_TARGET_BLEND.alpha.src_factor, wgpu::BlendFactor::One);
        assert_eq!(
            CLOUD_TARGET_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
    }

    #[test]
    fn cloud_packed_face_pipeline_validates_wgsl_on_wgpu_device() {
        let Some(device) = request_test_device() else {
            return;
        };
        let terrain_layout = crate::gpu::create_terrain_bind_group_layout(&device);
        let cloud_layout = create_cloud_bind_group_layout(&device);

        let _pipeline = create_cloud_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &terrain_layout,
            &cloud_layout,
            CloudShape::Fancy,
        );
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
    fn cloud_target_color_write_feeds_premultiplied_transparency_layer() {
        // `post/transparency.fsh` blends sorted layers as `dst * (1 - src.a) + src.rgb`;
        // clouds target RGB becomes premultiplied through the translucent cloud pass.
        assert_eq!(
            CLOUD_TARGET_BLEND.color.src_factor,
            wgpu::BlendFactor::SrcAlpha
        );
        assert_eq!(
            CLOUD_TARGET_BLEND.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(CLOUD_TARGET_BLEND.alpha.src_factor, wgpu::BlendFactor::One);
        assert_eq!(
            CLOUD_TARGET_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
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
    fn vanilla_cloud_radius_matches_configured_chunk_range_and_cell_size() {
        let environment = CloudEnvironment::overworld_default().with_range_chunks(64);
        assert_eq!(
            cloud_placement(
                environment,
                &create_cloud_texture_data(&CloudTextureImage {
                    width: 1,
                    height: 1,
                    rgba: vec![0, 0, 0, 255],
                })
                .unwrap(),
                CloudFrame::default(),
                CloudShape::Fancy,
            )
            .mesh_key
            .radius_cells,
            (64.0 * 16.0_f32 / CLOUD_CELL_SIZE_IN_BLOCKS).ceil() as i32
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
    fn cloud_vertices_keep_basic_plane_extent_equal_to_configured_range() {
        let vertices =
            basic_cloud_plane_vertices(CloudEnvironment::overworld_default().with_range_chunks(64));

        assert_eq!(
            vertices[0].position,
            [-(64.0 * 16.0), VANILLA_DEFAULT_CLOUD_HEIGHT, -(64.0 * 16.0),]
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
            cloud_uniform(CloudEnvironment::overworld_default(), frame, Some(&texture)).offset,
            [
                frame.camera_position[0] + placement.offset[0],
                VANILLA_DEFAULT_CLOUD_HEIGHT,
                frame.camera_position[2] + placement.offset[1],
                0.0
            ]
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
    fn cloud_face_record_preserves_vanilla_three_byte_encoding() {
        let record = encode_cloud_face(-3, 4, CloudFace::East, true, false);

        assert_eq!(std::mem::size_of::<CloudFaceRecord>(), 4);
        assert_eq!(record.packed.to_le_bytes(), [0xfe, 0x02, 0x95, 0x00]);
        let flat = encode_cloud_face(2, -1, CloudFace::Down, false, true);
        assert_eq!(flat.packed.to_le_bytes(), [0x01, 0xff, 0x60, 0x00]);
        assert_eq!(CLOUD_QUAD_INDICES, [0, 1, 2, 2, 3, 0]);
    }

    #[test]
    fn cloud_shader_consumes_vanilla_cloud_fog_end_slot() {
        assert!(CLOUD_SHADER.contains("let cloud_end = camera.fog_visibility_ends.y;"));
        assert!(CLOUD_SHADER.contains("@group(1) @binding(0)"));
        assert!(CLOUD_SHADER.contains("@location(0) packed_face: u32"));
        assert!(CLOUD_SHADER.contains("let directed_vertex = select("));
        assert!(CLOUD_SHADER.contains("+ cloud_info.offset.xyz"));
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
