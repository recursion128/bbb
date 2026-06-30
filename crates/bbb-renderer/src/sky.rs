use anyhow::{anyhow, bail, Result};
use glam::Vec3;
use wgpu::util::DeviceExt;

const SKY_DISC_RADIUS: f32 = 512.0;
const SKY_DISC_Y: f32 = 16.0;
const SUNRISE_STEPS: usize = 16;
const SUNRISE_CENTER_Y: f32 = 100.0;
const SUNRISE_RING_RADIUS: f32 = 120.0;
const SUNRISE_RING_DEPTH: f32 = 40.0;
const SUNRISE_ALPHA_EPSILON: f32 = 0.001;
const END_SKY_HALF_EXTENT: f32 = 100.0;
const END_SKY_UV_REPEAT: f32 = 16.0;
const END_SKY_VERTEX_COLOR: [f32; 4] = [40.0 / 255.0, 40.0 / 255.0, 40.0 / 255.0, 1.0];
const CELESTIAL_HEIGHT: f32 = 100.0;
const CELESTIAL_SUN_SIZE: f32 = 30.0;
const CELESTIAL_MOON_SIZE: f32 = 20.0;
const CELESTIAL_TEXTURE_COUNT: usize = 9;
const STAR_RANDOM_SEED: i64 = 10_842;
const STAR_SAMPLE_COUNT: usize = 1_500;
const STAR_DISTANCE: f32 = 100.0;
const STAR_MIN_LENGTH_SQUARED: f32 = 0.010_000_001;
const VANILLA_ACCEPTED_STAR_QUADS: usize = 780;
const JAVA_RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const JAVA_RANDOM_INCREMENT: u64 = 11;
const JAVA_RANDOM_MASK: u64 = (1_u64 << 48) - 1;

const SKY_OVERLAY_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::Zero,
        operation: wgpu::BlendOperation::Add,
    },
};

fn sky_depth_stencil_state() -> Option<wgpu::DepthStencilState> {
    None
}

fn sky_cull_mode() -> Option<wgpu::Face> {
    Some(wgpu::Face::Back)
}

fn sky_disc_blend_state() -> Option<wgpu::BlendState> {
    None
}

fn sunrise_sunset_blend_state() -> Option<wgpu::BlendState> {
    Some(wgpu::BlendState::ALPHA_BLENDING)
}

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

const SKY_TEXTURED_SHADER: &str = r#"
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
@group(1) @binding(0)
var sky_texture: texture_2d<f32>;
@group(1) @binding(1)
var sky_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world_position = input.position + camera.camera_position.xyz;
    out.position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(sky_texture, sky_sampler, input.uv) * input.color;
    if texel.a <= 0.0 {
        discard;
    }
    return texel;
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyboxKind {
    None,
    Overworld,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyMoonPhase {
    FullMoon,
    WaningGibbous,
    ThirdQuarter,
    WaningCrescent,
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
}

impl SkyMoonPhase {
    pub const ALL: [Self; 8] = [
        Self::FullMoon,
        Self::WaningGibbous,
        Self::ThirdQuarter,
        Self::WaningCrescent,
        Self::NewMoon,
        Self::WaxingCrescent,
        Self::FirstQuarter,
        Self::WaxingGibbous,
    ];

    pub fn from_vanilla_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn vanilla_index(self) -> usize {
        match self {
            Self::FullMoon => 0,
            Self::WaningGibbous => 1,
            Self::ThirdQuarter => 2,
            Self::WaningCrescent => 3,
            Self::NewMoon => 4,
            Self::WaxingCrescent => 5,
            Self::FirstQuarter => 6,
            Self::WaxingGibbous => 7,
        }
    }

    fn texture_kind(self) -> CelestialTextureKind {
        match self {
            Self::FullMoon => CelestialTextureKind::MoonFull,
            Self::WaningGibbous => CelestialTextureKind::MoonWaningGibbous,
            Self::ThirdQuarter => CelestialTextureKind::MoonThirdQuarter,
            Self::WaningCrescent => CelestialTextureKind::MoonWaningCrescent,
            Self::NewMoon => CelestialTextureKind::MoonNew,
            Self::WaxingCrescent => CelestialTextureKind::MoonWaxingCrescent,
            Self::FirstQuarter => CelestialTextureKind::MoonFirstQuarter,
            Self::WaxingGibbous => CelestialTextureKind::MoonWaxingGibbous,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CelestialTextureKind {
    Sun,
    MoonFull,
    MoonWaningGibbous,
    MoonThirdQuarter,
    MoonWaningCrescent,
    MoonNew,
    MoonWaxingCrescent,
    MoonFirstQuarter,
    MoonWaxingGibbous,
}

impl CelestialTextureKind {
    pub const ALL: [Self; CELESTIAL_TEXTURE_COUNT] = [
        Self::Sun,
        Self::MoonFull,
        Self::MoonWaningGibbous,
        Self::MoonThirdQuarter,
        Self::MoonWaningCrescent,
        Self::MoonNew,
        Self::MoonWaxingCrescent,
        Self::MoonFirstQuarter,
        Self::MoonWaxingGibbous,
    ];

    fn index(self) -> usize {
        match self {
            Self::Sun => 0,
            Self::MoonFull => 1,
            Self::MoonWaningGibbous => 2,
            Self::MoonThirdQuarter => 3,
            Self::MoonWaningCrescent => 4,
            Self::MoonNew => 5,
            Self::MoonWaxingCrescent => 6,
            Self::MoonFirstQuarter => 7,
            Self::MoonWaxingGibbous => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CelestialTextureImage {
    pub kind: CelestialTextureKind,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyEnvironment {
    pub skybox: SkyboxKind,
    pub color: [f32; 4],
    pub sunrise_sunset_color: [f32; 4],
    pub sun_angle_radians: f32,
    pub moon_angle_radians: f32,
    pub rain_brightness: f32,
    pub moon_phase: SkyMoonPhase,
    pub star_angle_radians: f32,
    pub star_brightness: f32,
}

impl Default for SkyEnvironment {
    fn default() -> Self {
        Self::disabled()
    }
}

impl SkyEnvironment {
    pub fn disabled() -> Self {
        Self {
            skybox: SkyboxKind::None,
            color: [0.0, 0.0, 0.0, 0.0],
            sunrise_sunset_color: [0.0, 0.0, 0.0, 0.0],
            sun_angle_radians: 0.0,
            moon_angle_radians: std::f32::consts::PI,
            rain_brightness: 0.0,
            moon_phase: SkyMoonPhase::FullMoon,
            star_angle_radians: 0.0,
            star_brightness: 0.0,
        }
    }

    pub fn end() -> Self {
        Self {
            skybox: SkyboxKind::End,
            color: [0.0, 0.0, 0.0, 0.0],
            sunrise_sunset_color: [0.0, 0.0, 0.0, 0.0],
            sun_angle_radians: 0.0,
            moon_angle_radians: std::f32::consts::PI,
            rain_brightness: 0.0,
            moon_phase: SkyMoonPhase::FullMoon,
            star_angle_radians: 0.0,
            star_brightness: 0.0,
        }
    }

    pub fn from_rgb(color: [f32; 3]) -> Self {
        Self {
            skybox: SkyboxKind::Overworld,
            color: [color[0], color[1], color[2], 1.0],
            sunrise_sunset_color: [0.0, 0.0, 0.0, 0.0],
            sun_angle_radians: 0.0,
            moon_angle_radians: std::f32::consts::PI,
            rain_brightness: 1.0,
            moon_phase: SkyMoonPhase::FullMoon,
            star_angle_radians: 0.0,
            star_brightness: 0.0,
        }
        .sanitized()
    }

    pub fn with_sunrise_sunset(mut self, color: [f32; 4], sun_angle_radians: f32) -> Self {
        self.sunrise_sunset_color = color;
        self.sun_angle_radians = sun_angle_radians;
        self.sanitized()
    }

    pub fn with_celestial_state(
        mut self,
        moon_angle_radians: f32,
        rain_brightness: f32,
        moon_phase: SkyMoonPhase,
    ) -> Self {
        self.moon_angle_radians = moon_angle_radians;
        self.rain_brightness = rain_brightness;
        self.moon_phase = moon_phase;
        self.sanitized()
    }

    pub fn with_star_state(mut self, star_angle_radians: f32, star_brightness: f32) -> Self {
        self.star_angle_radians = star_angle_radians;
        self.star_brightness = star_brightness;
        self.sanitized()
    }

    pub fn sanitized(self) -> Self {
        Self {
            skybox: self.skybox,
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
            moon_angle_radians: sanitize_radians(self.moon_angle_radians),
            rain_brightness: sanitize_unit(self.rain_brightness),
            moon_phase: self.moon_phase,
            star_angle_radians: sanitize_radians(self.star_angle_radians),
            star_brightness: sanitize_unit(self.star_brightness),
        }
    }

    pub fn is_visible(self) -> bool {
        let environment = self.sanitized();
        environment.skybox == SkyboxKind::Overworld && environment.color[3] > 0.0
    }

    pub fn sunrise_sunset_visible(self) -> bool {
        let environment = self.sanitized();
        environment.skybox == SkyboxKind::Overworld
            && environment.sunrise_sunset_color[3] > SUNRISE_ALPHA_EPSILON
    }

    pub fn end_sky_visible(self) -> bool {
        self.sanitized().skybox == SkyboxKind::End
    }

    pub fn celestials_visible(self) -> bool {
        let environment = self.sanitized();
        environment.skybox == SkyboxKind::Overworld && environment.rain_brightness > 0.0
    }

    pub fn stars_visible(self) -> bool {
        let environment = self.sanitized();
        environment.skybox == SkyboxKind::Overworld && environment.star_brightness > 0.0
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyVertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyTexturedVertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

pub(super) struct SkyDiscGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) disc_vertex_count: u32,
    pub(super) sunrise_vertex_start: u32,
    pub(super) sunrise_vertex_count: u32,
}

pub(super) struct EndSkyGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
}

pub(super) struct EndSkyTextureGpu {
    pub(super) _texture: wgpu::Texture,
    pub(super) _view: wgpu::TextureView,
    pub(super) _sampler: wgpu::Sampler,
    pub(super) bind_group: wgpu::BindGroup,
}

pub(super) struct CelestialGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
}

pub(super) struct CelestialAtlasGpu {
    pub(super) _texture: wgpu::Texture,
    pub(super) _view: wgpu::TextureView,
    pub(super) _sampler: wgpu::Sampler,
    pub(super) bind_group: wgpu::BindGroup,
    uvs: [Option<SkyTextureUvRect>; CELESTIAL_TEXTURE_COUNT],
}

pub(super) struct StarGpu {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SkyTextureUvRect {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
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
                blend: sky_disc_blend_state(),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: sky_cull_mode(),
            ..Default::default()
        },
        depth_stencil: sky_depth_stencil_state(),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

pub(super) fn create_sunrise_sunset_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-sunrise-sunset-shader"),
        source: wgpu::ShaderSource::Wgsl(SKY_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-sunrise-sunset-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-sunrise-sunset-pipeline"),
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
                blend: sunrise_sunset_blend_state(),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: sky_cull_mode(),
            ..Default::default()
        },
        depth_stencil: sky_depth_stencil_state(),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

pub(super) fn create_star_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-star-shader"),
        source: wgpu::ShaderSource::Wgsl(SKY_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-star-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-star-pipeline"),
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
                blend: Some(SKY_OVERLAY_BLEND),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: sky_cull_mode(),
            ..Default::default()
        },
        depth_stencil: sky_depth_stencil_state(),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

pub(super) fn create_end_sky_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-end-sky-texture-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

pub(super) fn create_celestial_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-celestial-texture-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

pub(super) fn create_end_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-end-sky-shader"),
        source: wgpu::ShaderSource::Wgsl(SKY_TEXTURED_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-end-sky-pipeline-layout"),
        bind_group_layouts: &[camera_bind_group_layout, texture_bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-end-sky-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SkyTexturedVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3,
                    1 => Float32x2,
                    2 => Float32x4
                ],
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
            cull_mode: sky_cull_mode(),
            ..Default::default()
        },
        depth_stencil: sky_depth_stencil_state(),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

pub(super) fn create_celestial_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-celestial-shader"),
        source: wgpu::ShaderSource::Wgsl(SKY_TEXTURED_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-celestial-pipeline-layout"),
        bind_group_layouts: &[camera_bind_group_layout, texture_bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-celestial-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SkyTexturedVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3,
                    1 => Float32x2,
                    2 => Float32x4
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(SKY_OVERLAY_BLEND),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: sky_cull_mode(),
            ..Default::default()
        },
        depth_stencil: sky_depth_stencil_state(),
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
    let batch = sky_vertex_batch(environment);
    if batch.vertices.is_empty() {
        return None;
    }
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-sky-disc-vertices"),
        contents: bytemuck::cast_slice(&batch.vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    Some(SkyDiscGpu {
        vertex_buffer,
        disc_vertex_count: batch.disc_vertex_count,
        sunrise_vertex_start: batch.sunrise_vertex_start,
        sunrise_vertex_count: batch.sunrise_vertex_count,
    })
}

pub(super) fn create_end_sky_gpu(device: &wgpu::Device) -> EndSkyGpu {
    let vertices = end_sky_vertices();
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-end-sky-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    EndSkyGpu {
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    }
}

pub(super) fn create_celestial_gpu(
    device: &wgpu::Device,
    environment: SkyEnvironment,
    atlas: &CelestialAtlasGpu,
) -> Option<CelestialGpu> {
    let vertices = celestial_vertices(environment, &atlas.uvs);
    if vertices.is_empty() {
        return None;
    }
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-celestial-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    Some(CelestialGpu {
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    })
}

pub(super) fn create_star_gpu(
    device: &wgpu::Device,
    environment: SkyEnvironment,
) -> Option<StarGpu> {
    let vertices = star_vertices(environment);
    if vertices.is_empty() {
        return None;
    }
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-star-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    Some(StarGpu {
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    })
}

pub(super) fn create_end_sky_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<EndSkyTextureGpu> {
    validate_end_sky_rgba(width, height, rgba)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-end-sky-texture"),
        size: wgpu::Extent3d {
            width,
            height,
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
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-end-sky-sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-end-sky-texture-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok(EndSkyTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

pub(super) fn create_celestial_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    images: &[CelestialTextureImage],
) -> Result<CelestialAtlasGpu> {
    let atlas = pack_celestial_atlas(images)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-celestial-atlas"),
        size: wgpu::Extent3d {
            width: atlas.width,
            height: atlas.height,
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
        &atlas.rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(atlas.width * 4),
            rows_per_image: Some(atlas.height),
        },
        wgpu::Extent3d {
            width: atlas.width,
            height: atlas.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-celestial-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-celestial-texture-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok(CelestialAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        uvs: atlas.uvs,
    })
}

struct SkyVertexBatch {
    vertices: Vec<SkyVertex>,
    disc_vertex_count: u32,
    sunrise_vertex_start: u32,
    sunrise_vertex_count: u32,
}

fn sky_vertex_batch(environment: SkyEnvironment) -> SkyVertexBatch {
    let mut vertices = Vec::new();
    if environment.is_visible() {
        vertices.extend(sky_disc_vertices(environment.color));
    }
    let disc_vertex_count = vertices.len() as u32;
    vertices.extend(sunrise_sunset_vertices(
        environment.sunrise_sunset_color,
        environment.sun_angle_radians,
    ));
    let sunrise_vertex_start = disc_vertex_count;
    let sunrise_vertex_count = vertices.len() as u32 - sunrise_vertex_start;
    SkyVertexBatch {
        vertices,
        disc_vertex_count,
        sunrise_vertex_start,
        sunrise_vertex_count,
    }
}

fn end_sky_vertices() -> Vec<SkyTexturedVertex> {
    let mut vertices = Vec::with_capacity(6 * 6);
    for face in 0..6 {
        let quad = end_sky_face_vertices(face);
        vertices.extend([quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
    }
    vertices
}

fn end_sky_face_vertices(face: usize) -> [SkyTexturedVertex; 4] {
    let positions = [
        [
            -END_SKY_HALF_EXTENT,
            -END_SKY_HALF_EXTENT,
            -END_SKY_HALF_EXTENT,
        ],
        [
            -END_SKY_HALF_EXTENT,
            -END_SKY_HALF_EXTENT,
            END_SKY_HALF_EXTENT,
        ],
        [
            END_SKY_HALF_EXTENT,
            -END_SKY_HALF_EXTENT,
            END_SKY_HALF_EXTENT,
        ],
        [
            END_SKY_HALF_EXTENT,
            -END_SKY_HALF_EXTENT,
            -END_SKY_HALF_EXTENT,
        ],
    ];
    let uvs = [
        [0.0, 0.0],
        [0.0, END_SKY_UV_REPEAT],
        [END_SKY_UV_REPEAT, END_SKY_UV_REPEAT],
        [END_SKY_UV_REPEAT, 0.0],
    ];

    std::array::from_fn(|index| SkyTexturedVertex {
        position: rotate_end_sky_face(face, positions[index]),
        uv: uvs[index],
        color: END_SKY_VERTEX_COLOR,
    })
}

fn celestial_vertices(
    environment: SkyEnvironment,
    uvs: &[Option<SkyTextureUvRect>; CELESTIAL_TEXTURE_COUNT],
) -> Vec<SkyTexturedVertex> {
    let environment = environment.sanitized();
    if !environment.celestials_visible() {
        return Vec::new();
    }

    let Some(sun_uv) = uvs[CelestialTextureKind::Sun.index()] else {
        return Vec::new();
    };
    let Some(moon_uv) = uvs[environment.moon_phase.texture_kind().index()] else {
        return Vec::new();
    };

    let mut vertices = Vec::with_capacity(12);
    vertices.extend(celestial_quad_vertices(
        CELESTIAL_SUN_SIZE,
        environment.sun_angle_radians,
        sun_uv,
        CelestialUvOrientation::Sun,
        environment.rain_brightness,
    ));
    vertices.extend(celestial_quad_vertices(
        CELESTIAL_MOON_SIZE,
        environment.moon_angle_radians,
        moon_uv,
        CelestialUvOrientation::Moon,
        environment.rain_brightness,
    ));
    vertices
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CelestialUvOrientation {
    Sun,
    Moon,
}

fn celestial_quad_vertices(
    size: f32,
    angle_radians: f32,
    uv_rect: SkyTextureUvRect,
    orientation: CelestialUvOrientation,
    alpha: f32,
) -> [SkyTexturedVertex; 6] {
    let base_positions = [
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
    ];
    let uvs = match orientation {
        CelestialUvOrientation::Sun => [
            [uv_rect.u0, uv_rect.v0],
            [uv_rect.u1, uv_rect.v0],
            [uv_rect.u1, uv_rect.v1],
            [uv_rect.u0, uv_rect.v1],
        ],
        CelestialUvOrientation::Moon => [
            [uv_rect.u1, uv_rect.v1],
            [uv_rect.u0, uv_rect.v1],
            [uv_rect.u0, uv_rect.v0],
            [uv_rect.u1, uv_rect.v0],
        ],
    };
    let quad: [SkyTexturedVertex; 4] = std::array::from_fn(|index| SkyTexturedVertex {
        position: celestial_position(base_positions[index], size, angle_radians),
        uv: uvs[index],
        color: [1.0, 1.0, 1.0, alpha],
    });
    [quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]
}

fn celestial_position(position: [f32; 3], size: f32, angle_radians: f32) -> [f32; 3] {
    let local = [
        position[0] * size,
        CELESTIAL_HEIGHT + position[1],
        position[2] * size,
    ];
    rotate_y(rotate_x(local, angle_radians), -std::f32::consts::FRAC_PI_2)
}

fn star_vertices(environment: SkyEnvironment) -> Vec<SkyVertex> {
    let environment = environment.sanitized();
    if !environment.stars_visible() {
        return Vec::new();
    }
    let color = [
        environment.star_brightness,
        environment.star_brightness,
        environment.star_brightness,
        environment.star_brightness,
    ];
    base_star_vertices()
        .into_iter()
        .map(|position| SkyVertex {
            position: rotate_y(
                rotate_x(position, environment.star_angle_radians),
                -std::f32::consts::FRAC_PI_2,
            ),
            color,
        })
        .collect()
}

fn base_star_vertices() -> Vec<[f32; 3]> {
    let mut random = JavaRandom::new(STAR_RANDOM_SEED);
    let mut vertices = Vec::with_capacity(VANILLA_ACCEPTED_STAR_QUADS * 6);
    for _ in 0..STAR_SAMPLE_COUNT {
        let x = random.next_float() * 2.0 - 1.0;
        let y = random.next_float() * 2.0 - 1.0;
        let z = random.next_float() * 2.0 - 1.0;
        let star_size = 0.15 + random.next_float() * 0.1;
        let length_squared = x * x + y * y + z * z;
        if length_squared <= STAR_MIN_LENGTH_SQUARED || length_squared >= 1.0 {
            continue;
        }

        let center = Vec3::new(x, y, z).normalize() * STAR_DISTANCE;
        let z_rotation = random.next_double() as f32 * std::f32::consts::TAU;
        let quad = star_quad_vertices(center, star_size, z_rotation);
        vertices.extend([quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
    }
    vertices
}

fn star_quad_vertices(center: Vec3, star_size: f32, z_rotation: f32) -> [[f32; 3]; 4] {
    let normal = -center.normalize();
    let mut right = Vec3::Y.cross(normal);
    if right.length_squared() <= f32::EPSILON {
        right = Vec3::X.cross(normal);
    }
    right = right.normalize();
    let up = normal.cross(right).normalize();
    let local_positions = [
        [star_size, -star_size],
        [star_size, star_size],
        [-star_size, star_size],
        [-star_size, -star_size],
    ];

    std::array::from_fn(|index| {
        let [x, y] = rotate_star_local_z(local_positions[index], -z_rotation);
        (center + right * x + up * y).to_array()
    })
}

fn rotate_star_local_z([x, y]: [f32; 2], radians: f32) -> [f32; 2] {
    let sin = radians.sin();
    let cos = radians.cos();
    [x * cos - y * sin, x * sin + y * cos]
}

#[derive(Debug, Clone, Copy)]
struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ JAVA_RANDOM_MULTIPLIER) & JAVA_RANDOM_MASK,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(JAVA_RANDOM_MULTIPLIER)
            .wrapping_add(JAVA_RANDOM_INCREMENT)
            & JAVA_RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_u32 << 24) as f32
    }

    fn next_double(&mut self) -> f64 {
        let high = self.next_bits(26) as u64;
        let low = self.next_bits(27) as u64;
        ((high << 27) | low) as f64 / (1_u64 << 53) as f64
    }
}

fn rotate_end_sky_face(face: usize, position: [f32; 3]) -> [f32; 3] {
    match face {
        1 => rotate_x(position, std::f32::consts::FRAC_PI_2),
        2 => rotate_x(position, -std::f32::consts::FRAC_PI_2),
        3 => rotate_x(position, std::f32::consts::PI),
        4 => rotate_z(position, std::f32::consts::FRAC_PI_2),
        5 => rotate_z(position, -std::f32::consts::FRAC_PI_2),
        _ => position,
    }
}

fn rotate_x([x, y, z]: [f32; 3], radians: f32) -> [f32; 3] {
    let sin = radians.sin();
    let cos = radians.cos();
    [x, y * cos - z * sin, y * sin + z * cos]
}

fn rotate_y([x, y, z]: [f32; 3], radians: f32) -> [f32; 3] {
    let sin = radians.sin();
    let cos = radians.cos();
    [x * cos + z * sin, y, -x * sin + z * cos]
}

fn rotate_z([x, y, z]: [f32; 3], radians: f32) -> [f32; 3] {
    let sin = radians.sin();
    let cos = radians.cos();
    [x * cos - y * sin, x * sin + y * cos, z]
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

fn validate_end_sky_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("end sky texture dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("end sky texture size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "end sky texture has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

#[derive(Debug)]
struct PackedCelestialAtlas {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    uvs: [Option<SkyTextureUvRect>; CELESTIAL_TEXTURE_COUNT],
}

fn pack_celestial_atlas(images: &[CelestialTextureImage]) -> Result<PackedCelestialAtlas> {
    let images_by_kind = validate_celestial_texture_images(images)?;
    let atlas_width = images_by_kind
        .iter()
        .flatten()
        .try_fold(0_u32, |width, image| {
            width
                .checked_add(image.width)
                .ok_or_else(|| anyhow!("celestial atlas width overflow"))
        })?;
    let atlas_height = images_by_kind
        .iter()
        .flatten()
        .map(|image| image.height)
        .max()
        .unwrap_or(0);
    let atlas_len = usize::try_from(atlas_width)
        .ok()
        .and_then(|width| {
            usize::try_from(atlas_height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("celestial atlas size overflow"))?;
    let mut rgba = vec![0; atlas_len];
    let mut uvs = [None; CELESTIAL_TEXTURE_COUNT];
    let mut x_offset = 0_u32;

    for image in images_by_kind.iter().flatten() {
        copy_celestial_image_into_atlas(&mut rgba, atlas_width, image, x_offset)?;
        uvs[image.kind.index()] = Some(SkyTextureUvRect {
            u0: x_offset as f32 / atlas_width as f32,
            v0: 0.0,
            u1: (x_offset + image.width) as f32 / atlas_width as f32,
            v1: image.height as f32 / atlas_height as f32,
        });
        x_offset += image.width;
    }

    Ok(PackedCelestialAtlas {
        width: atlas_width,
        height: atlas_height,
        rgba,
        uvs,
    })
}

fn validate_celestial_texture_images(
    images: &[CelestialTextureImage],
) -> Result<[Option<&CelestialTextureImage>; CELESTIAL_TEXTURE_COUNT]> {
    let mut images_by_kind = [None; CELESTIAL_TEXTURE_COUNT];
    for image in images {
        validate_celestial_texture_image(image)?;
        let slot = &mut images_by_kind[image.kind.index()];
        if slot.is_some() {
            bail!("duplicate celestial texture {:?}", image.kind);
        }
        *slot = Some(image);
    }
    for kind in CelestialTextureKind::ALL {
        if images_by_kind[kind.index()].is_none() {
            bail!("missing celestial texture {:?}", kind);
        }
    }
    Ok(images_by_kind)
}

fn validate_celestial_texture_image(image: &CelestialTextureImage) -> Result<()> {
    if image.width == 0 || image.height == 0 {
        bail!(
            "celestial texture {:?} dimensions must be non-zero",
            image.kind
        );
    }
    let expected_len = usize::try_from(image.width)
        .ok()
        .and_then(|width| {
            usize::try_from(image.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("celestial texture {:?} size overflow", image.kind))?;
    if image.rgba.len() != expected_len {
        bail!(
            "celestial texture {:?} has {} RGBA bytes, expected {} for {}x{}",
            image.kind,
            image.rgba.len(),
            expected_len,
            image.width,
            image.height
        );
    }
    Ok(())
}

fn copy_celestial_image_into_atlas(
    atlas_rgba: &mut [u8],
    atlas_width: u32,
    image: &CelestialTextureImage,
    x_offset: u32,
) -> Result<()> {
    let atlas_width = usize::try_from(atlas_width)?;
    let x_offset = usize::try_from(x_offset)?;
    let image_width = usize::try_from(image.width)?;
    let image_height = usize::try_from(image.height)?;
    for y in 0..image_height {
        let src_start = y * image_width * 4;
        let src_end = src_start + image_width * 4;
        let dst_start = (y * atlas_width + x_offset) * 4;
        let dst_end = dst_start + image_width * 4;
        atlas_rgba[dst_start..dst_end].copy_from_slice(&image.rgba[src_start..src_end]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_environment_sanitizes_color_channels() {
        let environment = SkyEnvironment {
            skybox: SkyboxKind::Overworld,
            color: [1.5, -1.0, f32::NAN, 2.0],
            sunrise_sunset_color: [-1.0, 0.5, 2.0, f32::NAN],
            sun_angle_radians: f32::INFINITY,
            moon_angle_radians: f32::NEG_INFINITY,
            rain_brightness: 2.0,
            moon_phase: SkyMoonPhase::WaxingGibbous,
            star_angle_radians: f32::NAN,
            star_brightness: -1.0,
        }
        .sanitized();

        assert_eq!(environment.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(environment.sunrise_sunset_color, [0.0, 0.5, 1.0, 0.0]);
        assert_eq!(environment.sun_angle_radians, 0.0);
        assert_eq!(environment.moon_angle_radians, 0.0);
        assert_eq!(environment.rain_brightness, 1.0);
        assert_eq!(environment.moon_phase, SkyMoonPhase::WaxingGibbous);
        assert_eq!(environment.star_angle_radians, 0.0);
        assert_eq!(environment.star_brightness, 0.0);
        assert!(environment.is_visible());
        assert!(!SkyEnvironment::disabled().is_visible());
        assert!(SkyEnvironment::end().end_sky_visible());
        assert!(!SkyEnvironment::end().is_visible());
    }

    #[test]
    fn sky_pipelines_match_vanilla_disc_and_sunrise_state() {
        assert!(sky_depth_stencil_state().is_none());
        assert_eq!(sky_cull_mode(), Some(wgpu::Face::Back));
        assert!(sky_disc_blend_state().is_none());

        let sunrise_blend = sunrise_sunset_blend_state().expect("sunrise uses translucent blend");
        assert_eq!(sunrise_blend.color.src_factor, wgpu::BlendFactor::SrcAlpha);
        assert_eq!(
            sunrise_blend.color.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
        assert_eq!(sunrise_blend.alpha.src_factor, wgpu::BlendFactor::One);
        assert_eq!(
            sunrise_blend.alpha.dst_factor,
            wgpu::BlendFactor::OneMinusSrcAlpha
        );
    }

    #[test]
    fn end_sky_vertices_match_vanilla_quad_cube_shape() {
        let vertices = end_sky_vertices();

        assert_eq!(vertices.len(), 36);
        assert_eq!(
            vertices[0].position,
            [
                -END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT
            ]
        );
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[0].color, END_SKY_VERTEX_COLOR);
        assert_eq!(
            vertices[1].position,
            [
                -END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
                END_SKY_HALF_EXTENT
            ]
        );
        assert_eq!(vertices[1].uv, [0.0, END_SKY_UV_REPEAT]);
        assert_eq!(
            vertices[2].position,
            [
                END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
                END_SKY_HALF_EXTENT
            ]
        );
        assert_eq!(vertices[2].uv, [END_SKY_UV_REPEAT, END_SKY_UV_REPEAT]);
        assert_eq!(
            vertices[5].position,
            [
                END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT
            ]
        );
        assert_eq!(vertices[5].uv, [END_SKY_UV_REPEAT, 0.0]);
    }

    #[test]
    fn end_sky_vertices_apply_vanilla_face_rotations() {
        let face_x_pos = end_sky_face_vertices(1);
        let face_x_neg = end_sky_face_vertices(2);
        let face_z_pos = end_sky_face_vertices(4);
        let face_z_neg = end_sky_face_vertices(5);

        assert_close3(
            face_x_pos[0].position,
            [
                -END_SKY_HALF_EXTENT,
                END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
            ],
        );
        assert_close3(
            face_x_neg[0].position,
            [
                -END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
                END_SKY_HALF_EXTENT,
            ],
        );
        assert_close3(
            face_z_pos[0].position,
            [
                END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
            ],
        );
        assert_close3(
            face_z_neg[0].position,
            [
                -END_SKY_HALF_EXTENT,
                END_SKY_HALF_EXTENT,
                -END_SKY_HALF_EXTENT,
            ],
        );
    }

    #[test]
    fn sky_moon_phase_indices_follow_vanilla_order() {
        assert_eq!(SkyMoonPhase::FullMoon.vanilla_index(), 0);
        assert_eq!(SkyMoonPhase::WaningGibbous.vanilla_index(), 1);
        assert_eq!(SkyMoonPhase::ThirdQuarter.vanilla_index(), 2);
        assert_eq!(SkyMoonPhase::WaningCrescent.vanilla_index(), 3);
        assert_eq!(SkyMoonPhase::NewMoon.vanilla_index(), 4);
        assert_eq!(SkyMoonPhase::WaxingCrescent.vanilla_index(), 5);
        assert_eq!(SkyMoonPhase::FirstQuarter.vanilla_index(), 6);
        assert_eq!(SkyMoonPhase::WaxingGibbous.vanilla_index(), 7);
        assert_eq!(
            SkyMoonPhase::from_vanilla_index(9),
            SkyMoonPhase::WaningGibbous
        );
    }

    #[test]
    fn celestial_vertices_match_vanilla_sun_quad_transform_and_uvs() {
        let mut uvs = [None; CELESTIAL_TEXTURE_COUNT];
        uvs[CelestialTextureKind::Sun.index()] = Some(SkyTextureUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 0.25,
            v1: 0.5,
        });
        uvs[CelestialTextureKind::MoonFull.index()] = Some(SkyTextureUvRect {
            u0: 0.25,
            v0: 0.0,
            u1: 0.5,
            v1: 0.5,
        });
        let environment = SkyEnvironment::from_rgb([0.25, 0.5, 0.75])
            .with_sunrise_sunset([0.0, 0.0, 0.0, 0.0], 0.0)
            .with_celestial_state(0.0, 0.5, SkyMoonPhase::FullMoon);

        let vertices = celestial_vertices(environment, &uvs);

        assert_eq!(vertices.len(), 12);
        assert_close3(vertices[0].position, [30.0, 100.0, -30.0]);
        assert_close3(vertices[1].position, [30.0, 100.0, 30.0]);
        assert_close3(vertices[2].position, [-30.0, 100.0, 30.0]);
        assert_close2(vertices[0].uv, [0.0, 0.0]);
        assert_close2(vertices[1].uv, [0.25, 0.0]);
        assert_close2(vertices[2].uv, [0.25, 0.5]);
        assert_eq!(vertices[0].color, [1.0, 1.0, 1.0, 0.5]);
    }

    #[test]
    fn celestial_vertices_match_vanilla_moon_phase_uv_order_and_size() {
        let mut uvs = [None; CELESTIAL_TEXTURE_COUNT];
        uvs[CelestialTextureKind::Sun.index()] = Some(SkyTextureUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 0.25,
            v1: 0.5,
        });
        uvs[CelestialTextureKind::MoonWaningGibbous.index()] = Some(SkyTextureUvRect {
            u0: 0.25,
            v0: 0.0,
            u1: 0.5,
            v1: 0.5,
        });
        let environment = SkyEnvironment::from_rgb([0.25, 0.5, 0.75]).with_celestial_state(
            0.0,
            0.25,
            SkyMoonPhase::WaningGibbous,
        );

        let vertices = celestial_vertices(environment, &uvs);

        assert_eq!(vertices.len(), 12);
        assert_close3(vertices[6].position, [20.0, 100.0, -20.0]);
        assert_close3(vertices[7].position, [20.0, 100.0, 20.0]);
        assert_close3(vertices[8].position, [-20.0, 100.0, 20.0]);
        assert_close2(vertices[6].uv, [0.5, 0.5]);
        assert_close2(vertices[7].uv, [0.25, 0.5]);
        assert_close2(vertices[8].uv, [0.25, 0.0]);
        assert_eq!(vertices[6].color, [1.0, 1.0, 1.0, 0.25]);
    }

    #[test]
    fn celestial_vertices_skip_when_required_sprite_uv_is_missing() {
        let uvs = [None; CELESTIAL_TEXTURE_COUNT];
        let environment = SkyEnvironment::from_rgb([0.25, 0.5, 0.75]);

        assert!(celestial_vertices(environment, &uvs).is_empty());
    }

    #[test]
    fn celestial_atlas_requires_each_vanilla_sprite_once() {
        let mut images = test_celestial_images();
        images.pop();

        let err = pack_celestial_atlas(&images).unwrap_err();

        assert!(err.to_string().contains("missing celestial texture"));

        let mut duplicate = test_celestial_images();
        duplicate.push(celestial_image(CelestialTextureKind::Sun, 1, 1, 99));
        let err = pack_celestial_atlas(&duplicate).unwrap_err();
        assert!(err.to_string().contains("duplicate celestial texture Sun"));
    }

    #[test]
    fn celestial_atlas_packs_sprites_in_vanilla_phase_order() {
        let atlas = pack_celestial_atlas(&test_celestial_images()).unwrap();

        assert_eq!(atlas.width, CELESTIAL_TEXTURE_COUNT as u32);
        assert_eq!(atlas.height, 1);
        for (index, kind) in CelestialTextureKind::ALL.into_iter().enumerate() {
            let uv = atlas.uvs[kind.index()].unwrap();
            assert_close(uv.u0, index as f32 / CELESTIAL_TEXTURE_COUNT as f32);
            assert_close(uv.u1, (index + 1) as f32 / CELESTIAL_TEXTURE_COUNT as f32);
            assert_eq!(atlas.rgba[index * 4], index as u8);
        }
    }

    #[test]
    fn star_vertices_match_vanilla_seeded_count_center_and_brightness() {
        let environment = SkyEnvironment::from_rgb([0.25, 0.5, 0.75]).with_star_state(0.0, 0.5);

        let vertices = star_vertices(environment);

        assert_eq!(vertices.len(), VANILLA_ACCEPTED_STAR_QUADS * 6);
        assert_eq!(vertices[0].color, [0.5, 0.5, 0.5, 0.5]);
        let first_center = star_quad_center([vertices[0], vertices[1], vertices[2], vertices[5]]);
        assert_close3(first_center, [-47.698_66, 69.925_74, -53.246_868]);
    }

    #[test]
    fn star_vertices_skip_without_brightness_or_overworld_skybox() {
        assert!(star_vertices(SkyEnvironment::from_rgb([0.25, 0.5, 0.75])).is_empty());
        assert!(star_vertices(SkyEnvironment::end().with_star_state(0.0, 0.5)).is_empty());
    }

    #[test]
    fn end_sky_celestial_and_star_winding_faces_origin_for_vanilla_cull() {
        for face in 0..6 {
            let quad = end_sky_face_vertices(face);
            let normal = triangle_normal(quad[0].position, quad[1].position, quad[2].position);
            let center = quad_center([
                quad[0].position,
                quad[1].position,
                quad[2].position,
                quad[3].position,
            ]);
            assert!(
                normal.dot(center) < 0.0,
                "end sky face {face} must face inward for default back-face cull"
            );
        }

        let uv_rect = SkyTextureUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        };
        let celestial = celestial_quad_vertices(
            CELESTIAL_SUN_SIZE,
            0.0,
            uv_rect,
            CelestialUvOrientation::Sun,
            1.0,
        );
        let normal = triangle_normal(
            celestial[0].position,
            celestial[1].position,
            celestial[2].position,
        );
        let center = quad_center([
            celestial[0].position,
            celestial[1].position,
            celestial[2].position,
            celestial[5].position,
        ]);
        assert!(
            normal.dot(center) < 0.0,
            "celestial quads must face the camera origin for default back-face cull"
        );

        let star = base_star_vertices();
        let normal = triangle_normal(star[0], star[1], star[2]);
        let center = quad_center([star[0], star[1], star[2], star[5]]);
        assert!(
            normal.dot(center) < 0.0,
            "star quads must face the camera origin for default back-face cull"
        );
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
        let vertices = sky_vertex_batch(environment).vertices;

        assert_eq!(vertices.len(), 24 + SUNRISE_STEPS * 3);
        assert_eq!(vertices[0].color, environment.color);
        assert_eq!(vertices[24].color, environment.sunrise_sunset_color);
    }

    #[test]
    fn sky_vertex_batch_splits_vanilla_sky_and_sunrise_draw_ranges() {
        let environment = SkyEnvironment::from_rgb([0.25, 0.5, 0.75])
            .with_sunrise_sunset([1.0, 0.25, 0.0, 0.75], 0.0);
        let batch = sky_vertex_batch(environment);

        assert_eq!(batch.disc_vertex_count, 24);
        assert_eq!(batch.sunrise_vertex_start, batch.disc_vertex_count);
        assert_eq!(batch.sunrise_vertex_count, (SUNRISE_STEPS * 3) as u32);
        assert_eq!(
            batch.vertices.len() as u32,
            batch.disc_vertex_count + batch.sunrise_vertex_count
        );
        assert_eq!(batch.vertices[0].color, environment.color);
        assert_eq!(
            batch.vertices[batch.sunrise_vertex_start as usize].color,
            environment.sunrise_sunset_color
        );
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-4,
            "actual {actual} != expected {expected}"
        );
    }

    fn assert_close2(actual: [f32; 2], expected: [f32; 2]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_close(actual, expected);
        }
    }

    fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_close(actual, expected);
        }
    }

    fn star_quad_center(vertices: [SkyVertex; 4]) -> [f32; 3] {
        let mut center = [0.0; 3];
        for vertex in vertices {
            center[0] += vertex.position[0] * 0.25;
            center[1] += vertex.position[1] * 0.25;
            center[2] += vertex.position[2] * 0.25;
        }
        center
    }

    fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
        let a = Vec3::from_array(a);
        let b = Vec3::from_array(b);
        let c = Vec3::from_array(c);
        (b - a).cross(c - a).normalize()
    }

    fn quad_center(vertices: [[f32; 3]; 4]) -> Vec3 {
        let mut center = Vec3::ZERO;
        for vertex in vertices {
            center += Vec3::from_array(vertex) * 0.25;
        }
        center
    }

    fn test_celestial_images() -> Vec<CelestialTextureImage> {
        CelestialTextureKind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| celestial_image(kind, 1, 1, index as u8))
            .collect()
    }

    fn celestial_image(
        kind: CelestialTextureKind,
        width: u32,
        height: u32,
        red: u8,
    ) -> CelestialTextureImage {
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..width * height {
            rgba.extend([red, 0, 0, 255]);
        }
        CelestialTextureImage {
            kind,
            width,
            height,
            rgba,
        }
    }
}
