use anyhow::{anyhow, bail, Result};
use bbb_registry_codec::legacy_random::LegacyRandomSource;
use glam::{Mat4, Vec3};

use crate::{
    pipeline_builder::RenderPipelineBuilder,
    texture_sampling::{
        copy_rgba_image, create_nearest_clamp_sampler, create_standalone_rgba_texture,
        create_texture_bind_group, upload_rgba_texture, validate_rgba,
    },
    StandaloneTextureSampling,
};

mod resident;
pub(crate) use resident::{SkyGpu, SkyPipelines};

const SKY_DISC_RADIUS: f32 = 512.0;
const SKY_DISC_Y: f32 = 16.0;
/// Vanilla `SkyRenderer` builds the dark disc with `buildSkyDisc(-16.0F)` and
/// draws it under a `translate(0.0F, 12.0F, 0.0F)` model view, so the baked
/// disc sits 4 blocks below the camera.
const DARK_SKY_DISC_Y: f32 = -16.0 + 12.0;
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
    // Vanilla sky layers draw with depth test and depth write disabled, but
    // they run inside the main world pass, which always binds the shared depth
    // attachment — so the pipelines must declare its format to satisfy wgpu.
    Some(wgpu::DepthStencilState {
        format: crate::gpu::DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Always),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    })
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

const SKY_DISC_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    r#"
struct SkyDynamic {
    model_transform: mat4x4<f32>,
    color_modulator: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(1) @binding(0)
var<uniform> sky_dynamic: SkyDynamic;

struct VertexIn {
    @location(0) position: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) spherical_distance: f32,
    @location(1) cylindrical_distance: f32,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model_position = (camera.sky_model_view * (sky_dynamic.model_transform * vec4<f32>(input.position, 1.0))).xyz;
    out.position = camera.projection * vec4<f32>(model_position, 1.0);
    out.spherical_distance = length(input.position);
    out.cylindrical_distance = max(length(input.position.xz), abs(input.position.y));
    return out;
}

fn apply_sky_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let sky_end = camera.fog_visibility_ends.x;
    if (sky_end <= 0.0) {
        return color;
    }
    let fog_value = max(
        linear_fog_value(spherical_distance, 0.0, sky_end),
        select(0.0, 1.0, cylindrical_distance > sky_end),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return apply_sky_fog(sky_dynamic.color_modulator, input.spherical_distance, input.cylindrical_distance);
}
"#,
);

const SKY_COLOR_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    r#"
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
    out.position = camera.projection * (camera.sky_model_view * vec4<f32>(input.position, 1.0));
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#,
);

const STAR_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    r#"
struct SkyDynamic {
    model_transform: mat4x4<f32>,
    color_modulator: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(1) @binding(0)
var<uniform> sky_dynamic: SkyDynamic;

struct VertexIn {
    @location(0) position: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model_position = (camera.sky_model_view * (sky_dynamic.model_transform * vec4<f32>(input.position, 1.0))).xyz;
    out.position = camera.projection * vec4<f32>(model_position, 1.0);
    return out;
}

@fragment
fn fs_main(_input: VertexOut) -> @location(0) vec4<f32> {
    return sky_dynamic.color_modulator;
}
"#,
);

const END_SKY_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    r#"
struct SkyDynamic {
    model_transform: mat4x4<f32>,
    color_modulator: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(1) @binding(0)
var sky_texture: texture_2d<f32>;
@group(1) @binding(1)
var sky_sampler: sampler;
@group(2) @binding(0)
var<uniform> sky_dynamic: SkyDynamic;

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
    let model_position = (camera.sky_model_view * (sky_dynamic.model_transform * vec4<f32>(input.position, 1.0))).xyz;
    out.position = camera.projection * vec4<f32>(model_position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(sky_texture, sky_sampler, input.uv) * input.color;
    if texel.a == 0.0 {
        discard;
    }
    return texel * sky_dynamic.color_modulator;
}
"#,
);

const CELESTIAL_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    r#"
struct SkyDynamic {
    model_transform: mat4x4<f32>,
    color_modulator: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(1) @binding(0)
var sky_texture: texture_2d<f32>;
@group(1) @binding(1)
var sky_sampler: sampler;
@group(2) @binding(0)
var<uniform> sky_dynamic: SkyDynamic;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model_position = (camera.sky_model_view * (sky_dynamic.model_transform * vec4<f32>(input.position, 1.0))).xyz;
    out.position = camera.projection * vec4<f32>(model_position, 1.0);
    out.uv = input.uv;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(sky_texture, sky_sampler, input.uv);
    if texel.a == 0.0 {
        discard;
    }
    return texel * sky_dynamic.color_modulator;
}
"#,
);

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
    /// Vanilla `SkyRenderState.shouldRenderDarkDisc`: the below-horizon dark
    /// disc drawn after sun/moon/stars while the player's eye is under the
    /// level's horizon height.
    pub dark_disc: bool,
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
            dark_disc: false,
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
            dark_disc: false,
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
            dark_disc: false,
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

    pub fn with_dark_disc(mut self, dark_disc: bool) -> Self {
        self.dark_disc = dark_disc;
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
            dark_disc: self.dark_disc,
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
struct SkyPositionVertex {
    position: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUvVertex {
    position: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyTexturedVertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyDynamicUniform {
    model_transform: [[f32; 4]; 4],
    color_modulator: [f32; 4],
}

pub(crate) struct SkyDynamicGpu {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
    local_transform: Mat4,
    color_modulator: [f32; 4],
}

pub(crate) struct EndSkyTextureGpu {
    pub(crate) _texture: wgpu::Texture,
    pub(crate) _view: wgpu::TextureView,
    pub(crate) _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
}

pub(crate) struct CelestialAtlasGpu {
    pub(crate) _texture: wgpu::Texture,
    pub(crate) _view: wgpu::TextureView,
    pub(crate) _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    uvs: [Option<SkyTextureUvRect>; CELESTIAL_TEXTURE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SkyTextureUvRect {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

pub(crate) fn create_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedShaderPair>,
) -> Result<wgpu::RenderPipeline> {
    let Some(shaders) = shaders else {
        return Ok(create_embedded_sky_pipeline(
            device,
            format,
            bind_group_layout,
            dynamic_bind_group_layout,
        ));
    };
    create_selected_sky_pipeline(
        device,
        format,
        &[bind_group_layout, dynamic_bind_group_layout],
        "bbb-selected-sky-disc",
        shaders,
        &[sky_position_vertex_layout()],
        sky_disc_blend_state(),
    )
}

fn create_embedded_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-sky-disc-pipeline")
        .shader("bbb-sky-disc-shader", SKY_DISC_SHADER)
        .layout(
            "bbb-sky-disc-pipeline-layout",
            &[bind_group_layout, dynamic_bind_group_layout],
        )
        .vertex_buffers(&[sky_position_vertex_layout()])
        .color_target(format, sky_disc_blend_state())
        .cull_mode(sky_cull_mode())
        .depth_stencil(sky_depth_stencil_state())
        .build()
}

pub(crate) fn create_sunrise_sunset_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-sunrise-sunset-pipeline")
        .shader("bbb-sunrise-sunset-shader", SKY_COLOR_SHADER)
        .layout("bbb-sunrise-sunset-pipeline-layout", &[bind_group_layout])
        .vertex_buffers(&[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SkyVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
        }])
        .color_target(format, sunrise_sunset_blend_state())
        .cull_mode(sky_cull_mode())
        .depth_stencil(sky_depth_stencil_state())
        .build()
}

pub(crate) fn create_sunrise_sunset_core_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedPositionColorShaders>,
) -> Result<wgpu::RenderPipeline> {
    let Some(shaders) = shaders else {
        return Ok(create_sunrise_sunset_pipeline(
            device,
            format,
            bind_group_layout,
        ));
    };
    crate::core_shaders::create_selected_position_color_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-selected-sunrise-sunset",
        &shaders.sky_vertex,
        &shaders.fragment,
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SkyVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
        },
        sunrise_sunset_blend_state(),
        sky_cull_mode(),
        false,
        wgpu::CompareFunction::Always,
    )
}

pub(crate) fn create_star_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedShaderPair>,
) -> Result<wgpu::RenderPipeline> {
    let Some(shaders) = shaders else {
        return Ok(create_embedded_star_pipeline(
            device,
            format,
            camera_bind_group_layout,
            dynamic_bind_group_layout,
        ));
    };
    create_selected_sky_pipeline(
        device,
        format,
        &[camera_bind_group_layout, dynamic_bind_group_layout],
        "bbb-selected-stars",
        shaders,
        &[sky_position_vertex_layout()],
        Some(SKY_OVERLAY_BLEND),
    )
}

fn create_embedded_star_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-star-pipeline")
        .shader("bbb-star-shader", STAR_SHADER)
        .layout(
            "bbb-star-pipeline-layout",
            &[camera_bind_group_layout, dynamic_bind_group_layout],
        )
        .vertex_buffers(&[sky_position_vertex_layout()])
        .color_target(format, Some(SKY_OVERLAY_BLEND))
        .cull_mode(sky_cull_mode())
        .depth_stencil(sky_depth_stencil_state())
        .build()
}

pub(crate) fn create_sky_dynamic_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-sky-dynamic-bind-group-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

fn create_sky_dynamic_gpu(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    model_transform: Mat4,
    color_modulator: [f32; 4],
) -> SkyDynamicGpu {
    let uniform = sky_dynamic_uniform(model_transform, color_modulator);
    let buffer = crate::frame_buffers::create_initialized_buffer(
        device,
        Some("bbb-sky-dynamic-uniform"),
        bytemuck::bytes_of(&uniform),
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-sky-dynamic-bind-group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    SkyDynamicGpu {
        buffer,
        bind_group,
        local_transform: model_transform,
        color_modulator,
    }
}

fn sky_dynamic_uniform(model_transform: Mat4, color_modulator: [f32; 4]) -> SkyDynamicUniform {
    SkyDynamicUniform {
        model_transform: model_transform.to_cols_array_2d(),
        color_modulator,
    }
}

impl SkyDynamicGpu {
    fn set(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut crate::frame_buffers::StagedCommands,
        local_transform: Mat4,
        color_modulator: [f32; 4],
    ) {
        if self.local_transform == local_transform && self.color_modulator == color_modulator {
            return;
        }
        self.local_transform = local_transform;
        self.color_modulator = color_modulator;
        uploads.write(
            device,
            &self.buffer,
            0,
            bytemuck::bytes_of(&sky_dynamic_uniform(local_transform, color_modulator)),
        );
    }

    pub(crate) fn invalidate(&mut self) {
        self.color_modulator = [f32::NAN; 4];
    }
}

pub(crate) fn create_end_sky_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

pub(crate) fn create_celestial_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

pub(crate) fn create_end_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedShaderPair>,
) -> Result<wgpu::RenderPipeline> {
    let Some(shaders) = shaders else {
        return Ok(create_embedded_end_sky_pipeline(
            device,
            format,
            camera_bind_group_layout,
            texture_bind_group_layout,
            dynamic_bind_group_layout,
        ));
    };
    create_selected_sky_pipeline(
        device,
        format,
        &[
            camera_bind_group_layout,
            texture_bind_group_layout,
            dynamic_bind_group_layout,
        ],
        "bbb-selected-end-sky",
        shaders,
        &[sky_textured_vertex_layout()],
        Some(wgpu::BlendState::ALPHA_BLENDING),
    )
}

fn create_embedded_end_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-end-sky-pipeline")
        .shader("bbb-end-sky-shader", END_SKY_SHADER)
        .layout(
            "bbb-end-sky-pipeline-layout",
            &[
                camera_bind_group_layout,
                texture_bind_group_layout,
                dynamic_bind_group_layout,
            ],
        )
        .vertex_buffers(&[sky_textured_vertex_layout()])
        .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
        .cull_mode(sky_cull_mode())
        .depth_stencil(sky_depth_stencil_state())
        .build()
}

pub(crate) fn create_celestial_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedShaderPair>,
) -> Result<wgpu::RenderPipeline> {
    let Some(shaders) = shaders else {
        return Ok(create_embedded_celestial_pipeline(
            device,
            format,
            camera_bind_group_layout,
            texture_bind_group_layout,
            dynamic_bind_group_layout,
        ));
    };
    create_selected_sky_pipeline(
        device,
        format,
        &[
            camera_bind_group_layout,
            texture_bind_group_layout,
            dynamic_bind_group_layout,
        ],
        "bbb-selected-celestial",
        shaders,
        &[sky_uv_vertex_layout()],
        Some(SKY_OVERLAY_BLEND),
    )
}

fn create_embedded_celestial_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    dynamic_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-celestial-pipeline")
        .shader("bbb-celestial-shader", CELESTIAL_SHADER)
        .layout(
            "bbb-celestial-pipeline-layout",
            &[
                camera_bind_group_layout,
                texture_bind_group_layout,
                dynamic_bind_group_layout,
            ],
        )
        .vertex_buffers(&[sky_uv_vertex_layout()])
        .color_target(format, Some(SKY_OVERLAY_BLEND))
        .cull_mode(sky_cull_mode())
        .depth_stencil(sky_depth_stencil_state())
        .build()
}

fn sky_position_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkyPositionVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    }
}

const SKY_UV_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

fn sky_uv_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkyUvVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &SKY_UV_VERTEX_ATTRIBUTES,
    }
}

const SKY_TEXTURED_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

fn sky_textured_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkyTexturedVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &SKY_TEXTURED_VERTEX_ATTRIBUTES,
    }
}

fn create_selected_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: &'static str,
    shaders: &crate::core_shaders::AdaptedShaderPair,
    vertex_layouts: &[wgpu::VertexBufferLayout<'_>],
    blend: Option<wgpu::BlendState>,
) -> Result<wgpu::RenderPipeline> {
    let (vertex, fragment) = crate::core_shaders::compile_selected_shader_stages(
        device,
        label,
        &shaders.vertex,
        &shaders.fragment,
    )?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-sky-pipeline-layout"),
        bind_group_layouts: &crate::pipeline_builder::optional_bind_group_layouts(
            bind_group_layouts,
        ),
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &crate::pipeline_builder::optional_vertex_layouts(vertex_layouts),
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: sky_cull_mode(),
                ..Default::default()
            },
            depth_stencil: sky_depth_stencil_state(),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
}

fn celestial_uvs(
    environment: SkyEnvironment,
    uvs: &[Option<SkyTextureUvRect>; CELESTIAL_TEXTURE_COUNT],
) -> Option<(SkyTextureUvRect, SkyTextureUvRect)> {
    let environment = environment.sanitized();
    if !environment.celestials_visible() {
        return None;
    }
    let sun_uv = uvs[CelestialTextureKind::Sun.index()]?;
    let moon_uv = uvs[environment.moon_phase.texture_kind().index()]?;
    Some((sun_uv, moon_uv))
}

pub(crate) fn create_end_sky_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
) -> Result<EndSkyTextureGpu> {
    let (texture, view, sampler) = create_standalone_rgba_texture(
        device,
        queue,
        "bbb-end-sky-texture",
        width,
        height,
        rgba,
        sampling,
    )?;
    let bind_group = create_texture_bind_group(
        device,
        layout,
        "bbb-end-sky-texture-bind-group",
        &view,
        &sampler,
    );

    Ok(EndSkyTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

const MAX_CELESTIAL_ATLAS_BYTES: usize = 32 * 1024 * 1024;
const DEFAULT_MAX_CELESTIAL_ATLAS_DIMENSION: u32 = 16_384;

pub(crate) fn create_celestial_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    images: &[CelestialTextureImage],
) -> Result<CelestialAtlasGpu> {
    let max_dimension = device.limits().max_texture_dimension_2d;
    let atlas = pack_celestial_atlas_with_limits(images, max_dimension, MAX_CELESTIAL_ATLAS_BYTES)?;
    let (texture, view) = upload_rgba_texture(
        device,
        queue,
        [atlas.width, atlas.height],
        &atlas.rgba,
        "bbb-celestial-atlas",
    )?;
    let sampler = create_nearest_clamp_sampler(device, "bbb-celestial-sampler");
    let bind_group = create_texture_bind_group(
        device,
        layout,
        "bbb-celestial-texture-bind-group",
        &view,
        &sampler,
    );

    Ok(CelestialAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        uvs: atlas.uvs,
    })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CelestialUvOrientation {
    Sun,
    Moon,
}

fn celestial_quad_vertices(
    uv_rect: SkyTextureUvRect,
    orientation: CelestialUvOrientation,
) -> [SkyUvVertex; 6] {
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
    let quad: [SkyUvVertex; 4] = std::array::from_fn(|index| SkyUvVertex {
        position: base_positions[index],
        uv: uvs[index],
    });
    [quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]
}

fn celestial_model_transform(size: f32, angle_radians: f32) -> Mat4 {
    Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(angle_radians)
        * Mat4::from_translation(Vec3::new(0.0, CELESTIAL_HEIGHT, 0.0))
        * Mat4::from_scale(Vec3::new(size, 1.0, size))
}

fn star_vertices() -> Vec<SkyPositionVertex> {
    base_star_vertices()
        .into_iter()
        .map(|position| SkyPositionVertex { position })
        .collect()
}

fn star_model_transform(star_angle_radians: f32) -> Mat4 {
    Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2) * Mat4::from_rotation_x(star_angle_radians)
}

fn base_star_vertices() -> Vec<[f32; 3]> {
    let mut random = LegacyRandomSource::with_seed(STAR_RANDOM_SEED);
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

#[cfg(test)]
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

fn sky_disc_vertices() -> Vec<SkyPositionVertex> {
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
        vertices.push(SkyPositionVertex { position: center });
        vertices.push(SkyPositionVertex { position: edge[0] });
        vertices.push(SkyPositionVertex { position: edge[1] });
    }
    vertices
}

/// Vanilla `buildSkyDisc(-16.0F)`: `signum(yy) * 512.0F` mirrors the rim's x
/// axis for the bottom disc; the +12 draw translate is baked into the y.
fn dark_sky_disc_vertices() -> Vec<SkyPositionVertex> {
    let mut ring = Vec::with_capacity(9);
    for degrees in (-180..=180).step_by(45) {
        let radians = (degrees as f32).to_radians();
        ring.push([
            -SKY_DISC_RADIUS * radians.cos(),
            DARK_SKY_DISC_Y,
            SKY_DISC_RADIUS * radians.sin(),
        ]);
    }

    let center = [0.0, DARK_SKY_DISC_Y, 0.0];
    let mut vertices = Vec::with_capacity((ring.len() - 1) * 3);
    for edge in ring.windows(2) {
        vertices.push(SkyPositionVertex { position: center });
        vertices.push(SkyPositionVertex { position: edge[0] });
        vertices.push(SkyPositionVertex { position: edge[1] });
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

#[derive(Debug)]
struct PackedCelestialAtlas {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    uvs: [Option<SkyTextureUvRect>; CELESTIAL_TEXTURE_COUNT],
}

#[cfg_attr(not(test), allow(dead_code))]
fn pack_celestial_atlas(images: &[CelestialTextureImage]) -> Result<PackedCelestialAtlas> {
    pack_celestial_atlas_with_limits(
        images,
        DEFAULT_MAX_CELESTIAL_ATLAS_DIMENSION,
        MAX_CELESTIAL_ATLAS_BYTES,
    )
}

fn pack_celestial_atlas_with_limits(
    images: &[CelestialTextureImage],
    max_dimension: u32,
    max_bytes: usize,
) -> Result<PackedCelestialAtlas> {
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
    if atlas_width > max_dimension || atlas_height > max_dimension {
        bail!(
            "celestial atlas dimensions {atlas_width}x{atlas_height} exceed device limit {max_dimension}"
        );
    }
    let atlas_len = usize::try_from(atlas_width)
        .ok()
        .and_then(|width| {
            usize::try_from(atlas_height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("celestial atlas size overflow"))?;
    if atlas_len > max_bytes {
        bail!("celestial atlas size {atlas_len} bytes exceeds {max_bytes} byte budget");
    }
    let mut rgba = vec![0; atlas_len];
    let mut uvs = [None; CELESTIAL_TEXTURE_COUNT];
    let mut x_offset = 0_u32;

    for image in images_by_kind.iter().flatten() {
        copy_rgba_image(
            &mut rgba,
            atlas_width,
            x_offset,
            0,
            image.width,
            &image.rgba,
        )?;
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
        validate_rgba(image.width, image.height, &image.rgba)?;
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
            dark_disc: false,
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
        let depth = sky_depth_stencil_state().expect("sky declares the main pass depth format");
        assert_eq!(depth.format, crate::gpu::DEPTH_FORMAT);
        assert_eq!(depth.depth_write_enabled, Some(false));
        assert_eq!(depth.depth_compare, Some(wgpu::CompareFunction::Always));
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
    fn sky_disc_shader_uses_vanilla_sky_fog_end_shape() {
        assert!(SKY_DISC_SHADER.contains("struct SkyDynamic"));
        assert!(SKY_DISC_SHADER.contains("model_transform: mat4x4<f32>"));
        assert!(SKY_DISC_SHADER.contains("@location(0) position: vec3<f32>"));
        assert!(!SKY_DISC_SHADER.contains("@location(1) color"));
        assert!(SKY_DISC_SHADER.contains("camera.projection * vec4<f32>(model_position, 1.0)"));
        assert!(SKY_DISC_SHADER
            .contains("apply_sky_fog(sky_dynamic.color_modulator, input.spherical_distance"));
        assert!(SKY_DISC_SHADER.contains("let sky_end = camera.fog_visibility_ends.x;"));
        assert!(SKY_DISC_SHADER.contains("linear_fog_value(spherical_distance, 0.0, sky_end)"));
        assert!(SKY_DISC_SHADER.contains("mix(color.rgb, camera.fog_color.rgb"));
        assert!(
            !SKY_COLOR_SHADER.contains("apply_sky_fog")
                && !SKY_COLOR_SHADER.contains("fog_visibility_ends.x"),
            "sunrise/sunset and stars mirror vanilla position_color/stars without sky fog"
        );
    }

    #[test]
    fn sky_fog_pixels_match_equal_endpoint_reference_at_boundary() {
        let Some(renderer) = crate::Renderer::new_offscreen(3, 1) else {
            return;
        };
        let start = SKY_DISC_SHADER.find("fn apply_sky_fog(").unwrap();
        let end = start + SKY_DISC_SHADER[start..].find("\n@fragment").unwrap();
        let mut source = concat!(
            crate::wgsl::camera_full!(),
            crate::wgsl::linear_fog_value!(),
            crate::wgsl::fullscreen_triangle!(),
            "@group(0) @binding(0) var<uniform> camera: Camera;\n",
        )
        .to_owned();
        source.push_str(&SKY_DISC_SHADER[start..end]);
        source.push_str(
            r#"
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let cylindrical_distance = 0.75 + floor(input.position.x) * 0.25;
    return apply_sky_fog(vec4<f32>(0.2, 0.4, 0.6, 0.25), 0.25, cylindrical_distance);
}
"#,
        );
        // Equal runtime endpoints keep FXC from specializing the reference to division by zero.
        let reference = source.replace(
            "select(0.0, 1.0, cylindrical_distance > sky_end)",
            "linear_fog_value(cylindrical_distance, sky_end, camera.fog_visibility_ends.y)",
        );
        let layout = create_test_camera_bind_group_layout(&renderer.device);
        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bbb-sky-fog-pixel-test"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(renderer.camera_buffers.world()),
                }],
            });
        let pipelines = [&source, &reference].map(|shader| {
            RenderPipelineBuilder::new(&renderer.device, "bbb-sky-fog-pixel-test")
                .shader("bbb-sky-fog-pixel-test", shader)
                .layout("bbb-sky-fog-pixel-test", &[&layout])
                .color_target(renderer.config.format, None)
                .build()
        });
        let output = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-sky-fog-pixel-test"),
            size: wgpu::Extent3d {
                width: 3,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renderer.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = output.create_view(&Default::default());
        for (sky_end, expected) in [
            (
                1.0,
                [[89u8, 89, 121, 64], [89, 89, 121, 64], [204, 51, 26, 64]],
            ),
            (0.0, [[51u8, 102, 153, 64]; 3]),
        ] {
            let camera = crate::camera::CameraUniform::identity().with_fog_environment(
                crate::FogEnvironment {
                    color: [0.8, 0.2, 0.1, 1.0],
                    sky_end,
                    cloud_end: sky_end,
                    ..crate::FogEnvironment::disabled()
                },
            );
            renderer.queue.write_buffer(
                renderer.camera_buffers.world().buffer,
                renderer.camera_buffers.world().offset,
                bytemuck::bytes_of(&camera),
            );
            let mut frames = Vec::new();
            for pipeline in &pipelines {
                let mut encoder = renderer.device.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("bbb-sky-fog-pixel-test"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.draw(0..3, 0..1);
                }
                let pending = renderer
                    .prepare_screenshot_copy(&mut encoder, &output)
                    .unwrap();
                renderer.queue.submit(Some(encoder.finish()));
                frames.push(renderer.read_screenshot_pixels(pending).unwrap());
            }
            assert_eq!(frames[0].rgba, frames[1].rgba);
            for (x, expected) in expected.into_iter().enumerate() {
                let actual = frames[0].pixel(x as u32, 0);
                eprintln!("sky fog end {sky_end}, boundary pixel {x}: {actual:?}");
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(&a, b)| a.abs_diff(b) <= 1),
                    "sky fog end {sky_end}, boundary pixel {x}: {actual:?}"
                );
            }
        }
    }

    #[test]
    fn star_shader_uses_vanilla_color_modulator_shape() {
        assert!(STAR_SHADER.contains("struct SkyDynamic"));
        assert!(STAR_SHADER.contains("model_transform: mat4x4<f32>"));
        assert!(STAR_SHADER.contains("@location(0) position: vec3<f32>"));
        assert!(!STAR_SHADER.contains("@location(1) color"));
        assert!(STAR_SHADER.contains("camera.projection * vec4<f32>(model_position, 1.0)"));
        assert!(STAR_SHADER.contains("return sky_dynamic.color_modulator;"));
    }

    #[test]
    fn end_sky_shader_uses_vanilla_position_tex_color_modulator_shape() {
        assert!(END_SKY_SHADER.contains("struct SkyDynamic"));
        assert!(END_SKY_SHADER.contains("model_transform: mat4x4<f32>"));
        assert!(END_SKY_SHADER.contains("@location(0) position: vec3<f32>"));
        assert!(END_SKY_SHADER.contains("@location(1) uv: vec2<f32>"));
        assert!(END_SKY_SHADER.contains("@location(2) color: vec4<f32>"));
        assert!(END_SKY_SHADER.contains("camera.projection * vec4<f32>(model_position, 1.0)"));
        assert!(END_SKY_SHADER
            .contains("textureSample(sky_texture, sky_sampler, input.uv) * input.color"));
        assert!(END_SKY_SHADER.contains("if texel.a == 0.0"));
        assert!(END_SKY_SHADER.contains("return texel * sky_dynamic.color_modulator;"));
    }

    #[test]
    fn celestial_shader_uses_vanilla_position_tex_color_modulator_shape() {
        assert!(CELESTIAL_SHADER.contains("struct SkyDynamic"));
        assert!(CELESTIAL_SHADER.contains("model_transform: mat4x4<f32>"));
        assert!(CELESTIAL_SHADER.contains("@location(0) position: vec3<f32>"));
        assert!(CELESTIAL_SHADER.contains("@location(1) uv: vec2<f32>"));
        assert!(!CELESTIAL_SHADER.contains("@location(2) color"));
        assert!(CELESTIAL_SHADER.contains("camera.projection * vec4<f32>(model_position, 1.0)"));
        assert!(CELESTIAL_SHADER.contains("if texel.a == 0.0"));
        assert!(CELESTIAL_SHADER.contains("return texel * sky_dynamic.color_modulator;"));
    }

    #[test]
    fn sky_color_pipelines_validate_wgsl_on_wgpu_device() {
        let Some(device) = request_test_device() else {
            return;
        };
        let camera_bind_group_layout = create_test_camera_bind_group_layout(&device);
        let sky_dynamic_bind_group_layout = create_sky_dynamic_bind_group_layout(&device);
        let end_sky_bind_group_layout = create_end_sky_bind_group_layout(&device);
        let celestial_bind_group_layout = create_celestial_bind_group_layout(&device);

        let _sky_pipeline = create_sky_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &camera_bind_group_layout,
            &sky_dynamic_bind_group_layout,
            None,
        )
        .expect("embedded sky pipeline");
        let _sunrise_pipeline = create_sunrise_sunset_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &camera_bind_group_layout,
        );
        let _star_pipeline = create_star_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &camera_bind_group_layout,
            &sky_dynamic_bind_group_layout,
            None,
        )
        .expect("embedded star pipeline");
        let _end_sky_pipeline = create_end_sky_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &camera_bind_group_layout,
            &end_sky_bind_group_layout,
            &sky_dynamic_bind_group_layout,
            None,
        )
        .expect("embedded end-sky pipeline");
        let _celestial_pipeline = create_celestial_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &camera_bind_group_layout,
            &celestial_bind_group_layout,
            &sky_dynamic_bind_group_layout,
            None,
        )
        .expect("embedded celestial pipeline");
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

        let (sun_uv, _) = celestial_uvs(environment, &uvs).unwrap();
        let vertices = celestial_quad_vertices(sun_uv, CelestialUvOrientation::Sun);
        let transform =
            celestial_model_transform(CELESTIAL_SUN_SIZE, environment.sun_angle_radians);

        assert_eq!(vertices.len(), 6);
        assert_close3(vertices[0].position, [-1.0, 0.0, -1.0]);
        assert_close3(vertices[1].position, [1.0, 0.0, -1.0]);
        assert_close3(vertices[2].position, [1.0, 0.0, 1.0]);
        assert_close3(
            transform
                .transform_point3(Vec3::from(vertices[0].position))
                .to_array(),
            [30.0, 100.0, -30.0],
        );
        assert_close3(
            transform
                .transform_point3(Vec3::from(vertices[1].position))
                .to_array(),
            [30.0, 100.0, 30.0],
        );
        assert_close3(
            transform
                .transform_point3(Vec3::from(vertices[2].position))
                .to_array(),
            [-30.0, 100.0, 30.0],
        );
        assert_close2(vertices[0].uv, [0.0, 0.0]);
        assert_close2(vertices[1].uv, [0.25, 0.0]);
        assert_close2(vertices[2].uv, [0.25, 0.5]);
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

        let (_, moon_uv) = celestial_uvs(environment, &uvs).unwrap();
        let vertices = celestial_quad_vertices(moon_uv, CelestialUvOrientation::Moon);
        let transform =
            celestial_model_transform(CELESTIAL_MOON_SIZE, environment.moon_angle_radians);

        assert_eq!(vertices.len(), 6);
        assert_close3(vertices[0].position, [-1.0, 0.0, -1.0]);
        assert_close3(vertices[1].position, [1.0, 0.0, -1.0]);
        assert_close3(vertices[2].position, [1.0, 0.0, 1.0]);
        assert_close3(
            transform
                .transform_point3(Vec3::from(vertices[0].position))
                .to_array(),
            [20.0, 100.0, -20.0],
        );
        assert_close3(
            transform
                .transform_point3(Vec3::from(vertices[1].position))
                .to_array(),
            [20.0, 100.0, 20.0],
        );
        assert_close3(
            transform
                .transform_point3(Vec3::from(vertices[2].position))
                .to_array(),
            [-20.0, 100.0, 20.0],
        );
        assert_close2(vertices[0].uv, [0.5, 0.5]);
        assert_close2(vertices[1].uv, [0.25, 0.5]);
        assert_close2(vertices[2].uv, [0.25, 0.0]);
    }

    #[test]
    fn celestial_vertices_skip_when_required_sprite_uv_is_missing() {
        let uvs = [None; CELESTIAL_TEXTURE_COUNT];
        let environment = SkyEnvironment::from_rgb([0.25, 0.5, 0.75]);

        assert!(celestial_uvs(environment, &uvs).is_none());
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
    fn star_vertices_match_vanilla_seeded_count_and_center() {
        let vertices = star_vertices();

        assert_eq!(vertices.len(), VANILLA_ACCEPTED_STAR_QUADS * 6);
        let first_center = star_quad_center([vertices[0], vertices[1], vertices[2], vertices[5]]);
        assert_close3(first_center, [-53.246_87, 69.925_74, 47.698_66]);
        let transformed = star_model_transform(0.0).transform_point3(Vec3::from(first_center));
        assert_close3(transformed.to_array(), [-47.698_66, 69.925_74, -53.246_87]);
    }

    #[test]
    fn star_vertices_keep_angle_in_dynamic_transform() {
        let base = star_vertices();
        let first_center = star_quad_center([base[0], base[1], base[2], base[5]]);
        let transformed = star_model_transform(1.25).transform_point3(Vec3::from(first_center));
        let baked = rotate_y(rotate_x(first_center, 1.25), -std::f32::consts::FRAC_PI_2);

        assert_close3(transformed.to_array(), baked);
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
        let celestial = celestial_quad_vertices(uv_rect, CelestialUvOrientation::Sun);
        let transform = celestial_model_transform(CELESTIAL_SUN_SIZE, 0.0);
        let transformed = celestial.map(|vertex| {
            transform
                .transform_point3(Vec3::from(vertex.position))
                .to_array()
        });
        let normal = triangle_normal(transformed[0], transformed[1], transformed[2]);
        let center = quad_center([
            transformed[0],
            transformed[1],
            transformed[2],
            transformed[5],
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
        let vertices = sky_disc_vertices();

        assert_eq!(vertices.len(), 24);
        assert_eq!(vertices[0].position, [0.0, SKY_DISC_Y, 0.0]);
        assert!((vertices[1].position[0] + SKY_DISC_RADIUS).abs() < 1e-4);
        assert_eq!(vertices[1].position[1], SKY_DISC_Y);
        assert!(vertices[1].position[2].abs() < 1e-4);
        assert!((vertices[23].position[0] + SKY_DISC_RADIUS).abs() < 1e-4);
        assert_eq!(vertices[23].position[1], SKY_DISC_Y);
        assert!(vertices[23].position[2].abs() < 1e-4);
    }

    #[test]
    fn dark_sky_disc_vertices_mirror_the_bottom_disc_with_the_baked_translate() {
        let vertices = dark_sky_disc_vertices();

        assert_eq!(vertices.len(), 24);
        // buildSkyDisc(-16) plus the renderDarkDisc translate (0, 12, 0).
        assert_eq!(DARK_SKY_DISC_Y, -4.0);
        assert_eq!(vertices[0].position, [0.0, DARK_SKY_DISC_Y, 0.0]);
        // signum(-16) mirrors the rim's x axis relative to the top disc.
        assert!((vertices[1].position[0] - SKY_DISC_RADIUS).abs() < 1e-4);
        assert_eq!(vertices[1].position[1], DARK_SKY_DISC_Y);
        assert!(vertices[1].position[2].abs() < 1e-4);
        assert!((vertices[23].position[0] - SKY_DISC_RADIUS).abs() < 1e-4);
        assert_eq!(vertices[23].position[1], DARK_SKY_DISC_Y);
        assert!(vertices[23].position[2].abs() < 1e-4);
    }

    #[test]
    fn sky_passes_follow_the_vanilla_sky_pass_gates() {
        let day = SkyEnvironment::from_rgb([0.5, 0.6, 0.7]);
        assert_eq!(
            resident::sky_passes(day),
            resident::SkyPasses {
                disc: true,
                celestials: true,
                ..Default::default()
            }
        );
        let dusk = day
            .with_sunrise_sunset([1.0, 0.25, 0.0, 0.75], 0.0)
            .with_star_state(0.0, 0.5)
            .with_dark_disc(true);
        assert_eq!(
            resident::sky_passes(dusk),
            resident::SkyPasses {
                end_sky: false,
                disc: true,
                sunrise: true,
                celestials: true,
                stars: true,
                dark_disc: true,
            }
        );
        // A disabled sky draws nothing, even with the dark disc gated on.
        assert_eq!(
            resident::sky_passes(SkyEnvironment::disabled().with_dark_disc(true)),
            resident::SkyPasses::default()
        );
        assert_eq!(
            resident::sky_passes(SkyEnvironment::end().with_star_state(0.0, 0.5)),
            resident::SkyPasses {
                end_sky: true,
                ..Default::default()
            }
        );
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

    fn star_quad_center(vertices: [SkyPositionVertex; 4]) -> [f32; 3] {
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

    fn request_test_device() -> Option<wgpu::Device> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            ..Default::default()
        }))
        .ok()?;
        let Ok((device, _queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bbb-sky-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            }))
        else {
            return None;
        };
        Some(device)
    }

    fn create_test_camera_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bbb-sky-test-camera-bind-group-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
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
