use std::{mem, ops::Range};

use anyhow::{anyhow, bail, Result};

use crate::gpu::DEPTH_FORMAT;

pub const WEATHER_RAIN_TEXTURE_PATH: &str = "textures/environment/rain.png";
pub const WEATHER_SNOW_TEXTURE_PATH: &str = "textures/environment/snow.png";

const WEATHER_TABLE_SIZE: usize = 32;
const WEATHER_TABLE_HALF: i32 = 16;
const RAIN_MAX_ALPHA: f32 = 1.0;
const SNOW_MAX_ALPHA: f32 = 0.8;

pub(crate) struct WeatherTextureGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherTextureKind {
    Rain,
    Snow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherTextureImage {
    pub kind: WeatherTextureKind,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherFrame {
    pub camera_position: [f32; 3],
    pub radius: u32,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherColumn {
    pub x: i32,
    pub z: i32,
    pub bottom_y: i32,
    pub top_y: i32,
    pub u_offset: f32,
    pub v_offset: f32,
    pub light: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeatherRenderState {
    pub frame: WeatherFrame,
    pub rain_columns: Vec<WeatherColumn>,
    pub snow_columns: Vec<WeatherColumn>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct WeatherVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
    pub(crate) light: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WeatherMesh {
    pub(crate) vertices: Vec<WeatherVertex>,
    pub(crate) indices: Vec<u32>,
    pub(crate) rain_indices: Range<u32>,
    pub(crate) snow_indices: Range<u32>,
}

const WEATHER_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2];

pub(crate) const WEATHER_SHADER: &str = r#"
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

@group(0) @binding(1)
var weather_texture: texture_2d<f32>;

@group(0) @binding(2)
var weather_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) spherical_distance: f32,
    @location(4) cylindrical_distance: f32,
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

fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let fog_value = max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}

fn sample_lightmap(light: vec2<f32>) -> vec3<f32> {
    let uv = clamp(
        light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0),
        vec2<f32>(0.5 / 16.0),
        vec2<f32>(15.5 / 16.0),
    );
    return textureSample(lightmap_texture, lightmap_sampler, uv).rgb;
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    out.light = input.light;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(weather_texture, weather_sampler, input.uv) * input.color;
    if (texel.a < 0.1) {
        discard;
    }
    let light_color = sample_lightmap(input.light);
    return apply_fog(vec4<f32>(texel.rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#;

impl Default for WeatherFrame {
    fn default() -> Self {
        Self {
            camera_position: [0.0, 0.0, 0.0],
            radius: 0,
            intensity: 0.0,
        }
    }
}

impl Default for WeatherRenderState {
    fn default() -> Self {
        Self {
            frame: WeatherFrame::default(),
            rain_columns: Vec::new(),
            snow_columns: Vec::new(),
        }
    }
}

impl WeatherFrame {
    pub fn at_camera_position(camera_position: [f32; 3], radius: u32, intensity: f32) -> Self {
        Self {
            camera_position: [
                sanitize_position(camera_position[0]),
                sanitize_position(camera_position[1]),
                sanitize_position(camera_position[2]),
            ],
            radius,
            intensity: sanitize_unit(intensity),
        }
    }
}

impl WeatherColumn {
    pub fn new(
        x: i32,
        z: i32,
        bottom_y: i32,
        top_y: i32,
        u_offset: f32,
        v_offset: f32,
        light: [f32; 2],
    ) -> Self {
        Self {
            x,
            z,
            bottom_y,
            top_y,
            u_offset: sanitize_offset(u_offset),
            v_offset: sanitize_offset(v_offset),
            light: [sanitize_unit(light[0]), sanitize_unit(light[1])],
        }
    }
}

impl WeatherRenderState {
    pub fn new(
        frame: WeatherFrame,
        rain_columns: Vec<WeatherColumn>,
        snow_columns: Vec<WeatherColumn>,
    ) -> Self {
        Self {
            frame,
            rain_columns,
            snow_columns,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.frame.intensity <= 0.0
            || self.frame.radius == 0
            || (self.rain_columns.is_empty() && self.snow_columns.is_empty())
    }

    pub fn rain_column_count(&self) -> usize {
        self.rain_columns.len()
    }

    pub fn snow_column_count(&self) -> usize {
        self.snow_columns.len()
    }
}

pub(crate) fn create_weather_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-weather-shader"),
        source: wgpu::ShaderSource::Wgsl(WEATHER_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-weather-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout, lightmap_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-weather-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[weather_vertex_layout()],
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
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

pub(crate) fn create_weather_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    image: &WeatherTextureImage,
) -> Result<WeatherTextureGpu> {
    validate_weather_texture_image(image)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(match image.kind {
            WeatherTextureKind::Rain => "bbb-weather-rain-texture",
            WeatherTextureKind::Snow => "bbb-weather-snow-texture",
        }),
        size: wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image.rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(image.width * 4),
            rows_per_image: Some(image.height),
        },
        wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(match image.kind {
            WeatherTextureKind::Rain => "bbb-weather-rain-sampler",
            WeatherTextureKind::Snow => "bbb-weather-snow-sampler",
        }),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(match image.kind {
            WeatherTextureKind::Rain => "bbb-weather-rain-bind-group",
            WeatherTextureKind::Snow => "bbb-weather-snow-bind-group",
        }),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok(WeatherTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

pub(crate) fn build_weather_mesh(state: &WeatherRenderState) -> Option<WeatherMesh> {
    if state.is_empty() {
        return None;
    }

    let mut vertices =
        Vec::with_capacity((state.rain_columns.len() + state.snow_columns.len()) * 4);
    let mut indices = Vec::with_capacity((state.rain_columns.len() + state.snow_columns.len()) * 6);
    let table = weather_column_size_table();

    append_columns(
        &mut vertices,
        &mut indices,
        &table,
        &state.rain_columns,
        state.frame,
        RAIN_MAX_ALPHA,
    );
    let rain_index_count = indices.len() as u32;
    append_columns(
        &mut vertices,
        &mut indices,
        &table,
        &state.snow_columns,
        state.frame,
        SNOW_MAX_ALPHA,
    );
    let total_index_count = indices.len() as u32;

    (!indices.is_empty()).then_some(WeatherMesh {
        vertices,
        indices,
        rain_indices: 0..rain_index_count,
        snow_indices: rain_index_count..total_index_count,
    })
}

fn append_columns(
    vertices: &mut Vec<WeatherVertex>,
    indices: &mut Vec<u32>,
    table: &[[f32; 2]; WEATHER_TABLE_SIZE * WEATHER_TABLE_SIZE],
    columns: &[WeatherColumn],
    frame: WeatherFrame,
    max_alpha: f32,
) {
    let radius_sq = frame.radius as f32 * frame.radius as f32;
    if radius_sq <= 0.0 {
        return;
    }

    for column in columns {
        if column.top_y == column.bottom_y {
            continue;
        }
        let Some([half_size_x, half_size_z]) = weather_column_half_size(table, column, frame)
        else {
            continue;
        };
        let relative_x = column.x as f32 + 0.5 - frame.camera_position[0];
        let relative_z = column.z as f32 + 0.5 - frame.camera_position[2];
        let distance_sq = relative_x * relative_x + relative_z * relative_z;
        let alpha =
            (max_alpha + (distance_sq / radius_sq).min(1.0) * (0.5 - max_alpha)) * frame.intensity;
        let color = [1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0)];
        let x0 = column.x as f32 + 0.5 - half_size_x;
        let x1 = column.x as f32 + 0.5 + half_size_x;
        let z0 = column.z as f32 + 0.5 - half_size_z;
        let z1 = column.z as f32 + 0.5 + half_size_z;
        let y0 = column.bottom_y as f32;
        let y1 = column.top_y as f32;
        let u0 = column.u_offset;
        let u1 = column.u_offset + 1.0;
        let v0 = column.bottom_y as f32 * 0.25 + column.v_offset;
        let v1 = column.top_y as f32 * 0.25 + column.v_offset;
        let base = vertices.len() as u32;

        vertices.extend_from_slice(&[
            weather_vertex([x0, y1, z0], [u0, v0], color, column.light),
            weather_vertex([x1, y1, z1], [u1, v0], color, column.light),
            weather_vertex([x1, y0, z1], [u1, v1], color, column.light),
            weather_vertex([x0, y0, z0], [u0, v1], color, column.light),
        ]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn weather_vertex(
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    light: [f32; 2],
) -> WeatherVertex {
    WeatherVertex {
        position,
        uv,
        color,
        light,
    }
}

fn weather_column_size_table() -> [[f32; 2]; WEATHER_TABLE_SIZE * WEATHER_TABLE_SIZE] {
    let mut table = [[0.0; 2]; WEATHER_TABLE_SIZE * WEATHER_TABLE_SIZE];
    for z in 0..WEATHER_TABLE_SIZE {
        for x in 0..WEATHER_TABLE_SIZE {
            let delta_x = x as f32 - WEATHER_TABLE_HALF as f32;
            let delta_z = z as f32 - WEATHER_TABLE_HALF as f32;
            let distance = (delta_x * delta_x + delta_z * delta_z).sqrt();
            table[z * WEATHER_TABLE_SIZE + x] = if distance > 0.0 {
                [-delta_z / distance, delta_x / distance]
            } else {
                [0.0, 0.0]
            };
        }
    }
    table
}

fn weather_column_half_size(
    table: &[[f32; 2]; WEATHER_TABLE_SIZE * WEATHER_TABLE_SIZE],
    column: &WeatherColumn,
    frame: WeatherFrame,
) -> Option<[f32; 2]> {
    let camera_x = frame.camera_position[0].floor() as i32;
    let camera_z = frame.camera_position[2].floor() as i32;
    let table_x = column.x - camera_x + WEATHER_TABLE_HALF;
    let table_z = column.z - camera_z + WEATHER_TABLE_HALF;
    if !(0..WEATHER_TABLE_SIZE as i32).contains(&table_x)
        || !(0..WEATHER_TABLE_SIZE as i32).contains(&table_z)
    {
        return None;
    }
    let [size_x, size_z] = table[table_z as usize * WEATHER_TABLE_SIZE + table_x as usize];
    Some([size_x * 0.5, size_z * 0.5])
}

fn validate_weather_texture_image(image: &WeatherTextureImage) -> Result<()> {
    if image.width == 0 || image.height == 0 {
        bail!("weather texture dimensions must be non-zero");
    }
    let expected_len = usize::try_from(image.width)
        .ok()
        .and_then(|width| {
            usize::try_from(image.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("weather texture size overflow"))?;
    if image.rgba.len() != expected_len {
        bail!(
            "weather texture has {} RGBA bytes, expected {} for {}x{}",
            image.rgba.len(),
            expected_len,
            image.width,
            image.height
        );
    }
    Ok(())
}

fn weather_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<WeatherVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &WEATHER_VERTEX_ATTRIBUTES,
    }
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

fn sanitize_offset(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000_01,
            "expected {actual} to be close to {expected}"
        );
    }

    #[test]
    fn weather_texture_paths_match_vanilla_environment_textures() {
        assert_eq!(WEATHER_RAIN_TEXTURE_PATH, "textures/environment/rain.png");
        assert_eq!(WEATHER_SNOW_TEXTURE_PATH, "textures/environment/snow.png");
    }

    #[test]
    fn weather_vertex_layout_matches_vanilla_particle_fields() {
        assert_eq!(WEATHER_VERTEX_ATTRIBUTES.len(), 4);
        assert_eq!(
            WEATHER_VERTEX_ATTRIBUTES[0].format,
            wgpu::VertexFormat::Float32x3
        );
        assert_eq!(
            WEATHER_VERTEX_ATTRIBUTES[1].format,
            wgpu::VertexFormat::Float32x2
        );
        assert_eq!(
            WEATHER_VERTEX_ATTRIBUTES[2].format,
            wgpu::VertexFormat::Float32x4
        );
        assert_eq!(
            WEATHER_VERTEX_ATTRIBUTES[3].format,
            wgpu::VertexFormat::Float32x2
        );
        assert!(WEATHER_SHADER.contains("@group(0) @binding(1)"));
        assert!(WEATHER_SHADER.contains("var weather_texture: texture_2d<f32>"));
        assert!(WEATHER_SHADER.contains("@group(1) @binding(0)"));
        assert!(WEATHER_SHADER.contains("var lightmap_texture: texture_2d<f32>"));
        assert!(WEATHER_SHADER.contains("@group(1) @binding(1)"));
        assert!(WEATHER_SHADER.contains("var lightmap_sampler: sampler"));
        assert!(WEATHER_SHADER.contains("light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0)"));
        assert!(WEATHER_SHADER.contains("if (texel.a < 0.1)"));
        assert!(WEATHER_SHADER.contains("texel.rgb * light_color"));
    }

    #[test]
    fn weather_mesh_emits_rain_then_snow_with_vanilla_column_math() {
        let frame = WeatherFrame::at_camera_position([0.25, 64.5, 0.25], 10, 0.5);
        let rain = WeatherColumn::new(2, 0, 60, 70, 0.0, 1.25, [0.2, 0.8]);
        let snow = WeatherColumn::new(0, 2, 61, 69, 0.5, -0.25, [0.6, 1.0]);
        let mesh = build_weather_mesh(&WeatherRenderState::new(frame, vec![rain], vec![snow]))
            .expect("weather mesh should contain rain and snow columns");

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        assert_eq!(mesh.rain_indices, 0..6);
        assert_eq!(mesh.snow_indices, 6..12);

        let rain_vertices = &mesh.vertices[..4];
        assert_eq!(rain_vertices[0].position, [2.5, 70.0, 0.0]);
        assert_eq!(rain_vertices[1].position, [2.5, 70.0, 1.0]);
        assert_eq!(rain_vertices[2].position, [2.5, 60.0, 1.0]);
        assert_eq!(rain_vertices[3].position, [2.5, 60.0, 0.0]);
        assert_eq!(rain_vertices[0].uv, [0.0, 16.25]);
        assert_eq!(rain_vertices[1].uv, [1.0, 16.25]);
        assert_eq!(rain_vertices[2].uv, [1.0, 18.75]);
        assert_eq!(rain_vertices[3].uv, [0.0, 18.75]);
        assert_eq!(rain_vertices[0].light, [0.2, 0.8]);
        assert_close(rain_vertices[0].color[3], 0.487_187_5);

        let snow_vertices = &mesh.vertices[4..];
        assert_eq!(snow_vertices[0].position, [1.0, 69.0, 2.5]);
        assert_eq!(snow_vertices[1].position, [0.0, 69.0, 2.5]);
        assert_eq!(snow_vertices[2].position, [0.0, 61.0, 2.5]);
        assert_eq!(snow_vertices[3].position, [1.0, 61.0, 2.5]);
        assert_close(snow_vertices[0].color[3], 0.392_312_5);
        assert_eq!(snow_vertices[0].light, [0.6, 1.0]);
    }

    #[test]
    fn invalid_weather_texture_dimensions_are_rejected() {
        let err = validate_weather_texture_image(&WeatherTextureImage {
            kind: WeatherTextureKind::Rain,
            width: 2,
            height: 2,
            rgba: vec![255; 4],
        })
        .unwrap_err();
        assert!(err.to_string().contains("expected 16 for 2x2"));
    }
}
