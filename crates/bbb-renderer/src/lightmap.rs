use crate::camera::LightmapEnvironment;

pub(super) const LIGHTMAP_TEXTURE_SIZE: u32 = 16;
pub(super) const LIGHTMAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub(super) const LIGHTMAP_SHADER: &str = r#"
struct LightmapInfo {
    SkyFactor: f32,
    BlockFactor: f32,
    NightVisionFactor: f32,
    DarknessScale: f32,
    BossOverlayWorldDarkeningFactor: f32,
    BrightnessFactor: f32,
    _padding0: vec2<f32>,
    BlockLightTint: vec4<f32>,
    SkyLightColor: vec4<f32>,
    AmbientColor: vec4<f32>,
    NightVisionColor: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> lightmapInfo: LightmapInfo;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var position = vec2<f32>(-1.0, -1.0);
    if (vertex_index == 1u) {
        position = vec2<f32>(3.0, -1.0);
    } else if (vertex_index == 2u) {
        position = vec2<f32>(-1.0, 3.0);
    }

    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.tex_coord = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

fn getBrightness(level: f32) -> f32 {
    return level / (4.0 - 3.0 * level);
}

fn notGamma(color: vec3<f32>) -> vec3<f32> {
    let maxComponent = max(max(color.x, color.y), color.z);
    if (maxComponent <= 0.0) {
        return color;
    }
    let maxInverted = 1.0 - maxComponent;
    let maxScaled = 1.0 - maxInverted * maxInverted * maxInverted * maxInverted;
    return color * (maxScaled / maxComponent);
}

fn parabolicMixFactor(level: f32) -> f32 {
    let centered = 2.0 * level - 1.0;
    return centered * centered;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let block_level = floor(input.tex_coord.x * 16.0) / 15.0;
    let sky_level = floor(input.tex_coord.y * 16.0) / 15.0;

    let block_brightness = getBrightness(block_level) * lightmapInfo.BlockFactor;
    let sky_brightness = getBrightness(sky_level) * lightmapInfo.SkyFactor;

    let nightVisionColor = lightmapInfo.NightVisionColor.rgb * lightmapInfo.NightVisionFactor;
    var color = max(lightmapInfo.AmbientColor.rgb, nightVisionColor);

    color += lightmapInfo.SkyLightColor.rgb * sky_brightness;

    let blockLightColor = mix(
        lightmapInfo.BlockLightTint.rgb,
        vec3<f32>(1.0),
        0.9 * parabolicMixFactor(block_level),
    );
    color += blockLightColor * block_brightness;

    color = mix(
        color,
        color * vec3<f32>(0.7, 0.6, 0.6),
        lightmapInfo.BossOverlayWorldDarkeningFactor,
    );
    color -= vec3<f32>(lightmapInfo.DarknessScale);

    let clamped = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    let notGammaColor = notGamma(clamped);
    color = mix(clamped, notGammaColor, lightmapInfo.BrightnessFactor);

    return vec4<f32>(color, 1.0);
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightmapUniform {
    sky_factor: f32,
    block_factor: f32,
    night_vision_factor: f32,
    darkness_scale: f32,
    boss_overlay_world_darkening_factor: f32,
    brightness_factor: f32,
    _padding0: [f32; 2],
    block_light_tint: [f32; 4],
    sky_light_color: [f32; 4],
    ambient_color: [f32; 4],
    night_vision_color: [f32; 4],
}

impl LightmapUniform {
    fn from_environment(environment: LightmapEnvironment) -> Self {
        let environment = environment.sanitized();
        Self {
            sky_factor: environment.sky_factor,
            block_factor: environment.block_factor,
            night_vision_factor: environment.night_vision_factor,
            darkness_scale: environment.darkness_scale,
            boss_overlay_world_darkening_factor: environment.boss_overlay_world_darkening,
            brightness_factor: environment.brightness_factor,
            _padding0: [0.0; 2],
            block_light_tint: rgb_to_vec4(environment.block_light_tint),
            sky_light_color: rgb_to_vec4(environment.sky_light_color),
            ambient_color: rgb_to_vec4(environment.ambient_color),
            night_vision_color: rgb_to_vec4(environment.night_vision_color),
        }
    }
}

pub(super) struct LightmapGpu {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) sample_bind_group: wgpu::BindGroup,
}

pub(super) fn create_lightmap_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-lightmap-bind-group-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub(super) fn create_lightmap_sample_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-lightmap-sample-bind-group-layout"),
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

pub(super) fn create_lightmap_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-lightmap-shader"),
        source: wgpu::ShaderSource::Wgsl(LIGHTMAP_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-lightmap-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-lightmap-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: LIGHTMAP_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

pub(super) fn create_lightmap_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    sample_bind_group_layout: &wgpu::BindGroupLayout,
    environment: LightmapEnvironment,
) -> LightmapGpu {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-lightmap-texture"),
        size: wgpu::Extent3d {
            width: LIGHTMAP_TEXTURE_SIZE,
            height: LIGHTMAP_TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LIGHTMAP_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let initial_white = [255u8; (LIGHTMAP_TEXTURE_SIZE * LIGHTMAP_TEXTURE_SIZE * 4) as usize];
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &initial_white,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * LIGHTMAP_TEXTURE_SIZE),
            rows_per_image: Some(LIGHTMAP_TEXTURE_SIZE),
        },
        wgpu::Extent3d {
            width: LIGHTMAP_TEXTURE_SIZE,
            height: LIGHTMAP_TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-lightmap-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bbb-lightmap-uniform-buffer"),
        size: std::mem::size_of::<LightmapUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    write_lightmap_uniform(queue, &uniform_buffer, environment);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-lightmap-bind-group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let sample_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-lightmap-sample-bind-group"),
        layout: sample_bind_group_layout,
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
    LightmapGpu {
        _texture: texture,
        view,
        _sampler: sampler,
        uniform_buffer,
        bind_group,
        sample_bind_group,
    }
}

pub(super) fn write_lightmap_uniform(
    queue: &wgpu::Queue,
    uniform_buffer: &wgpu::Buffer,
    environment: LightmapEnvironment,
) {
    let uniform = LightmapUniform::from_environment(environment);
    queue.write_buffer(uniform_buffer, 0, bytemuck::bytes_of(&uniform));
}

fn rgb_to_vec4(rgb: [f32; 3]) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], 0.0]
}

#[cfg(test)]
mod tests {
    use super::{LightmapUniform, LIGHTMAP_FORMAT, LIGHTMAP_SHADER, LIGHTMAP_TEXTURE_SIZE};
    use crate::camera::LightmapEnvironment;

    #[test]
    fn lightmap_texture_shape_matches_vanilla() {
        // Vanilla Lightmap creates TextureFormat.RGBA8 at 16x16.
        assert_eq!(LIGHTMAP_TEXTURE_SIZE, 16);
        assert_eq!(LIGHTMAP_FORMAT, wgpu::TextureFormat::Rgba8Unorm);
    }

    #[test]
    fn lightmap_uniform_matches_vanilla_lightmap_info_layout() {
        // Vanilla LightmapInfo is six scalar factors followed by four std140 vec3 colors.
        assert_eq!(std::mem::size_of::<LightmapUniform>(), 96);
        let uniform = LightmapUniform::from_environment(LightmapEnvironment {
            sky_factor: 0.25,
            block_factor: 1.5,
            night_vision_factor: 0.75,
            darkness_scale: 0.125,
            boss_overlay_world_darkening: 0.5,
            brightness_factor: 0.875,
            block_light_tint: [0.1, 0.2, 0.3],
            sky_light_color: [0.4, 0.5, 0.6],
            ambient_color: [0.7, 0.8, 0.9],
            night_vision_color: [0.9, 0.8, 0.7],
            level_lighting: crate::camera::LevelLighting::Nether,
        });
        assert_eq!(uniform.sky_factor, 0.25);
        assert_eq!(uniform.block_factor, 1.5);
        assert_eq!(uniform.night_vision_factor, 0.75);
        assert_eq!(uniform.darkness_scale, 0.125);
        assert_eq!(uniform.boss_overlay_world_darkening_factor, 0.5);
        assert_eq!(uniform.brightness_factor, 0.875);
        assert_eq!(uniform.block_light_tint, [0.1, 0.2, 0.3, 0.0]);
        assert_eq!(uniform.sky_light_color, [0.4, 0.5, 0.6, 0.0]);
        assert_eq!(uniform.ambient_color, [0.7, 0.8, 0.9, 0.0]);
        assert_eq!(uniform.night_vision_color, [0.9, 0.8, 0.7, 0.0]);
    }

    #[test]
    fn lightmap_shader_ports_vanilla_core_lightmap_formula() {
        assert!(LIGHTMAP_SHADER.contains("floor(input.tex_coord.x * 16.0) / 15.0"));
        assert!(LIGHTMAP_SHADER.contains("floor(input.tex_coord.y * 16.0) / 15.0"));
        assert!(LIGHTMAP_SHADER.contains("level / (4.0 - 3.0 * level)"));
        assert!(LIGHTMAP_SHADER
            .contains("lightmapInfo.NightVisionColor.rgb * lightmapInfo.NightVisionFactor"));
        assert!(LIGHTMAP_SHADER.contains("mix(\n        lightmapInfo.BlockLightTint.rgb"));
        assert!(LIGHTMAP_SHADER.contains("0.9 * parabolicMixFactor(block_level)"));
        assert!(LIGHTMAP_SHADER.contains("lightmapInfo.BossOverlayWorldDarkeningFactor"));
        assert!(LIGHTMAP_SHADER.contains("color -= vec3<f32>(lightmapInfo.DarknessScale)"));
        assert!(
            LIGHTMAP_SHADER.contains("mix(clamped, notGammaColor, lightmapInfo.BrightnessFactor)")
        );
    }
}
