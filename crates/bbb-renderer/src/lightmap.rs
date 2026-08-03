use crate::camera::LightmapEnvironment;
use crate::pipeline_builder::RenderPipelineBuilder;
use anyhow::Result;

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
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub(super) fn create_lightmap_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedLightmapShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return create_selected_lightmap_pipeline(device, bind_group_layout, shaders);
    }
    Ok(RenderPipelineBuilder::new(device, "bbb-lightmap-pipeline")
        .shader("bbb-lightmap-shader", LIGHTMAP_SHADER)
        .layout("bbb-lightmap-pipeline-layout", &[bind_group_layout])
        .color_target(LIGHTMAP_FORMAT, None)
        .build())
}

fn create_selected_lightmap_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: &crate::core_shaders::AdaptedLightmapShaders,
) -> Result<wgpu::RenderPipeline> {
    let (vertex, fragment) = crate::core_shaders::compile_selected_shader_stages(
        device,
        "bbb-selected-lightmap",
        &shaders.vertex,
        &shaders.fragment,
    )?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-lightmap-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bbb-selected-lightmap-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: "main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: LIGHTMAP_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        }),
    )
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
    use std::sync::mpsc;

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
        // Vanilla LightmapInfo has six scalar factors, eight bytes of std140
        // alignment padding, then four vec3 colors.
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
        assert_eq!(uniform._padding0, [0.0; 2]);
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
        assert!(!LIGHTMAP_SHADER.contains("AmbientLight"));
        assert!(LIGHTMAP_SHADER
            .contains("lightmapInfo.NightVisionColor.rgb * lightmapInfo.NightVisionFactor"));
        assert!(LIGHTMAP_SHADER.contains("mix(\n        lightmapInfo.BlockLightTint.rgb"));
        assert!(LIGHTMAP_SHADER.contains("0.9 * parabolicMixFactor(block_level)"));
        assert!(LIGHTMAP_SHADER.contains("lightmapInfo.BossOverlayWorldDarkeningFactor"));
        assert!(LIGHTMAP_SHADER.contains("color -= vec3<f32>(lightmapInfo.DarknessScale)"));
        assert!(
            LIGHTMAP_SHADER.contains("mix(clamped, notGammaColor, lightmapInfo.BrightnessFactor)")
        );

        let ambient = LIGHTMAP_SHADER
            .find("var color = max(lightmapInfo.AmbientColor.rgb, nightVisionColor)")
            .unwrap();
        let sky = LIGHTMAP_SHADER
            .find("color += lightmapInfo.SkyLightColor.rgb * sky_brightness")
            .unwrap();
        let block = LIGHTMAP_SHADER
            .find("color += blockLightColor * block_brightness")
            .unwrap();
        let boss = LIGHTMAP_SHADER
            .find("lightmapInfo.BossOverlayWorldDarkeningFactor")
            .unwrap();
        let darkness = LIGHTMAP_SHADER
            .find("color -= vec3<f32>(lightmapInfo.DarknessScale)")
            .unwrap();
        let clamp = LIGHTMAP_SHADER.find("let clamped = clamp(color").unwrap();
        let gamma = LIGHTMAP_SHADER
            .find("let notGammaColor = notGamma(clamped)")
            .unwrap();
        let brightness = LIGHTMAP_SHADER
            .find("mix(clamped, notGammaColor, lightmapInfo.BrightnessFactor)")
            .unwrap();
        assert!(
            ambient < sky
                && sky < block
                && block < boss
                && boss < darkness
                && darkness < clamp
                && clamp < gamma
                && gamma < brightness
        );
    }

    #[test]
    fn lightmap_gpu_matches_vanilla_formula() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            eprintln!("skipping lightmap GPU test without a compatible adapter");
            return;
        };
        let environment = LightmapEnvironment {
            sky_factor: 0.65,
            block_factor: 1.25,
            night_vision_factor: 0.3,
            darkness_scale: 0.08,
            boss_overlay_world_darkening: 0.4,
            brightness_factor: 0.55,
            block_light_tint: [1.0, 0.35, 0.1],
            sky_light_color: [0.2, 0.45, 0.9],
            ambient_color: [0.06, 0.04, 0.02],
            night_vision_color: [0.35, 0.8, 0.25],
            level_lighting: crate::camera::LevelLighting::Default,
        };

        let pixels = render_lightmap_pixels(&mut renderer, environment);
        for sky in 0..LIGHTMAP_TEXTURE_SIZE {
            for block in 0..LIGHTMAP_TEXTURE_SIZE {
                let expected = vanilla_lightmap_pixel(environment, block, sky);
                let offset = ((sky * LIGHTMAP_TEXTURE_SIZE + block) * 4) as usize;
                for channel in 0..4 {
                    assert!(
                        pixels[offset + channel].abs_diff(expected[channel]) <= 1,
                        "lightmap ({block}, {sky}) channel {channel}: actual {}, expected {}",
                        pixels[offset + channel],
                        expected[channel]
                    );
                }
            }
        }
    }

    #[test]
    fn lightmap_pass_is_consumed_once_per_environment_change() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            eprintln!("skipping lightmap dirty GPU test without a compatible adapter");
            return;
        };

        renderer
            .render_offscreen_frame()
            .expect("initial lightmap frame");
        let initial_draws = renderer.counters().draw_calls;
        renderer
            .render_offscreen_frame()
            .expect("unchanged lightmap frame");
        let unchanged_draws = renderer.counters().draw_calls;
        assert_eq!(
            unchanged_draws + 1,
            initial_draws,
            "only the dirty frame contains the standalone lightmap draw"
        );

        renderer.set_lightmap_brightness_factor(0.75);
        renderer
            .render_offscreen_frame()
            .expect("changed lightmap frame");
        assert_eq!(renderer.counters().draw_calls, initial_draws);
    }

    fn render_lightmap_pixels(
        renderer: &mut crate::Renderer,
        environment: LightmapEnvironment,
    ) -> Vec<u8> {
        renderer.set_lightmap_environment(environment);
        renderer.render_offscreen_frame().expect("offscreen frame");

        let unpadded_bytes_per_row = LIGHTMAP_TEXTURE_SIZE * 4;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-lightmap-test-readback"),
            size: u64::from(padded_bytes_per_row * LIGHTMAP_TEXTURE_SIZE),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-lightmap-test-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &renderer.lightmap._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(LIGHTMAP_TEXTURE_SIZE),
                },
            },
            wgpu::Extent3d {
                width: LIGHTMAP_TEXTURE_SIZE,
                height: LIGHTMAP_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        renderer.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let mapped = slice.get_mapped_range();
        let mut pixels =
            Vec::with_capacity((unpadded_bytes_per_row * LIGHTMAP_TEXTURE_SIZE) as usize);
        for row in mapped
            .chunks(padded_bytes_per_row as usize)
            .take(LIGHTMAP_TEXTURE_SIZE as usize)
        {
            pixels.extend_from_slice(&row[..unpadded_bytes_per_row as usize]);
        }
        pixels
    }

    fn vanilla_lightmap_pixel(environment: LightmapEnvironment, block: u32, sky: u32) -> [u8; 4] {
        let block_level = block as f32 / 15.0;
        let sky_level = sky as f32 / 15.0;
        let block_brightness = vanilla_lightmap_brightness(block_level) * environment.block_factor;
        let sky_brightness = vanilla_lightmap_brightness(sky_level) * environment.sky_factor;
        let mut color = [0.0; 3];
        for channel in 0..3 {
            color[channel] = environment.ambient_color[channel]
                .max(environment.night_vision_color[channel] * environment.night_vision_factor);
            color[channel] += environment.sky_light_color[channel] * sky_brightness;
            let parabolic = (2.0 * block_level - 1.0).powi(2);
            let block_tint = environment.block_light_tint[channel]
                + (1.0 - environment.block_light_tint[channel]) * 0.9 * parabolic;
            color[channel] += block_tint * block_brightness;
            color[channel] *=
                1.0 - environment.boss_overlay_world_darkening * (1.0 - [0.7, 0.6, 0.6][channel]);
            color[channel] = (color[channel] - environment.darkness_scale).clamp(0.0, 1.0);
        }

        let max_component = color.into_iter().fold(0.0_f32, f32::max);
        let max_inverted = 1.0 - max_component;
        let max_scaled = 1.0 - max_inverted.powi(4);
        for channel in 0..3 {
            let not_gamma = color[channel] * max_scaled / max_component;
            color[channel] += (not_gamma - color[channel]) * environment.brightness_factor;
        }
        [unorm8(color[0]), unorm8(color[1]), unorm8(color[2]), 255]
    }

    fn vanilla_lightmap_brightness(level: f32) -> f32 {
        level / (4.0 - 3.0 * level)
    }

    fn unorm8(value: f32) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}
