use crate::{
    clouds::CloudTarget,
    gpu::{DepthTarget, DEPTH_FORMAT, DEPTH_TARGET_USAGE},
    pipeline_builder::RenderPipelineBuilder,
};

pub(super) const TRANSPARENCY_COMBINE_SHADER: &str = r#"
@group(0) @binding(0)
var main_texture: texture_2d<f32>;

@group(0) @binding(1)
var main_depth: texture_depth_2d;

@group(0) @binding(2)
var translucent_texture: texture_2d<f32>;

@group(0) @binding(3)
var translucent_depth: texture_depth_2d;

@group(0) @binding(4)
var item_entity_texture: texture_2d<f32>;

@group(0) @binding(5)
var item_entity_depth: texture_depth_2d;

@group(0) @binding(6)
var particles_texture: texture_2d<f32>;

@group(0) @binding(7)
var particles_depth: texture_depth_2d;

@group(0) @binding(8)
var weather_texture: texture_2d<f32>;

@group(0) @binding(9)
var weather_depth: texture_depth_2d;

@group(0) @binding(10)
var clouds_texture: texture_2d<f32>;

@group(0) @binding(11)
var clouds_depth: texture_depth_2d;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
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
    out.uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

fn blend(dst: vec3<f32>, src: vec4<f32>) -> vec3<f32> {
    return (dst * (1.0 - src.a)) + src.rgb;
}

struct LayerStack {
    colors: array<vec4<f32>, 6>,
    depths: array<f32, 6>,
    active_count: u32,
};

fn try_insert(stack: LayerStack, color: vec4<f32>, depth: f32) -> LayerStack {
    var out = stack;
    if (color.a == 0.0) {
        return out;
    }

    out.colors[out.active_count] = color;
    out.depths[out.active_count] = depth;

    var jj = out.active_count;
    out.active_count = out.active_count + 1u;
    while (jj > 0u && out.depths[jj] > out.depths[jj - 1u]) {
        let ii = jj - 1u;
        let depth_temp = out.depths[ii];
        out.depths[ii] = out.depths[jj];
        out.depths[jj] = depth_temp;

        let color_temp = out.colors[ii];
        out.colors[ii] = out.colors[jj];
        out.colors[jj] = color_temp;

        jj = ii;
    }

    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(input.position.xy);
    var stack = LayerStack(
        array<vec4<f32>, 6>(
            vec4<f32>(textureLoad(main_texture, pixel, 0).rgb, 1.0),
            vec4<f32>(0.0),
            vec4<f32>(0.0),
            vec4<f32>(0.0),
            vec4<f32>(0.0),
            vec4<f32>(0.0),
        ),
        array<f32, 6>(
            textureLoad(main_depth, pixel, 0),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ),
        1u,
    );

    stack = try_insert(
        stack,
        textureLoad(translucent_texture, pixel, 0),
        textureLoad(translucent_depth, pixel, 0),
    );
    stack = try_insert(
        stack,
        textureLoad(item_entity_texture, pixel, 0),
        textureLoad(item_entity_depth, pixel, 0),
    );
    stack = try_insert(
        stack,
        textureLoad(particles_texture, pixel, 0),
        textureLoad(particles_depth, pixel, 0),
    );
    stack = try_insert(
        stack,
        textureLoad(weather_texture, pixel, 0),
        textureLoad(weather_depth, pixel, 0),
    );
    stack = try_insert(
        stack,
        textureLoad(clouds_texture, pixel, 0),
        textureLoad(clouds_depth, pixel, 0),
    );

    var texel_accum = stack.colors[0].rgb;
    var ii = 1u;
    while (ii < stack.active_count) {
        texel_accum = blend(texel_accum, stack.colors[ii]);
        ii = ii + 1u;
    }

    return vec4<f32>(texel_accum.rgb, 1.0);
}
"#;

pub(super) const TRANSPARENCY_BLIT_SHADER: &str = r#"
@group(0) @binding(0)
var final_texture: texture_2d<f32>;

@group(0) @binding(1)
var final_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
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
    out.uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(final_texture, final_sampler, input.uv);
}
"#;

pub(super) struct MainTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

pub(super) struct TranslucentTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) depth: DepthTarget,
}

pub(super) struct ItemEntityTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) depth: DepthTarget,
}

pub(super) struct ParticleTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) depth: DepthTarget,
}

pub(super) struct WeatherTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) depth: DepthTarget,
}

pub(super) struct TransparencyFinalTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) _sampler: wgpu::Sampler,
    pub(super) bind_group: wgpu::BindGroup,
}

pub(super) struct TransparencyCombineBindGroup {
    pub(super) bind_group: wgpu::BindGroup,
}

pub(super) fn create_transparency_blit_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-transparency-blit-bind-group-layout"),
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

pub(super) fn create_transparency_combine_bind_group_layout(
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-transparency-combine-bind-group-layout"),
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
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 8,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 9,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 10,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 11,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    })
}

pub(super) fn create_main_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> MainTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-main-target-color"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    MainTarget {
        _texture: texture,
        view,
    }
}

pub(super) fn create_transparency_final_target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> TransparencyFinalTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-transparency-final-target-color"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-transparency-final-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-transparency-final-bind-group"),
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

    TransparencyFinalTarget {
        _texture: texture,
        view,
        _sampler: sampler,
        bind_group,
    }
}

pub(super) fn create_translucent_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> TranslucentTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-translucent-target-color"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = create_translucent_depth_target(device, width, height);

    TranslucentTarget {
        _texture: texture,
        view,
        depth,
    }
}

fn create_translucent_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-translucent-target-depth"),
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

pub(super) fn create_item_entity_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> ItemEntityTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-entity-target-color"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = create_item_entity_depth_target(device, width, height);

    ItemEntityTarget {
        _texture: texture,
        view,
        depth,
    }
}

fn create_item_entity_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-entity-target-depth"),
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

pub(super) fn create_particle_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> ParticleTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-particle-target-color"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = create_particle_depth_target(device, width, height);

    ParticleTarget {
        _texture: texture,
        view,
        depth,
    }
}

fn create_particle_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-particle-target-depth"),
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

pub(super) fn create_weather_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> WeatherTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-weather-target-color"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = create_weather_depth_target(device, width, height);

    WeatherTarget {
        _texture: texture,
        view,
        depth,
    }
}

fn create_weather_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-weather-target-depth"),
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

pub(super) fn create_transparency_combine_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    main_target: &MainTarget,
    main_depth: &DepthTarget,
    translucent_target: &TranslucentTarget,
    item_entity_target: &ItemEntityTarget,
    particle_target: &ParticleTarget,
    weather_target: &WeatherTarget,
    cloud_target: &CloudTarget,
) -> TransparencyCombineBindGroup {
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-transparency-combine-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&main_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&main_depth.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&translucent_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&translucent_target.depth.view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&item_entity_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&item_entity_target.depth.view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&particle_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&particle_target.depth.view),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: wgpu::BindingResource::TextureView(&weather_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: wgpu::BindingResource::TextureView(&weather_target.depth.view),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: wgpu::BindingResource::TextureView(&cloud_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 11,
                resource: wgpu::BindingResource::TextureView(&cloud_target.depth.view),
            },
        ],
    });

    TransparencyCombineBindGroup { bind_group }
}

pub(super) fn create_transparency_combine_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_transparency_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-transparency-combine-shader",
        "bbb-transparency-combine-pipeline-layout",
        "bbb-transparency-combine-pipeline",
        TRANSPARENCY_COMBINE_SHADER,
    )
}

pub(super) fn create_transparency_blit_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_transparency_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-transparency-blit-shader",
        "bbb-transparency-blit-pipeline-layout",
        "bbb-transparency-blit-pipeline",
        TRANSPARENCY_BLIT_SHADER,
    )
}

fn create_transparency_fullscreen_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader_label: &str,
    pipeline_layout_label: &str,
    pipeline_label: &str,
    shader_source: &str,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, pipeline_label)
        .shader(shader_label, shader_source)
        .layout(pipeline_layout_label, &[bind_group_layout])
        .color_target(format, None)
        .build()
}

#[cfg(test)]
mod tests {
    use super::{
        create_transparency_blit_bind_group_layout, create_transparency_blit_pipeline,
        create_transparency_combine_bind_group_layout, create_transparency_combine_pipeline,
        TRANSPARENCY_BLIT_SHADER, TRANSPARENCY_COMBINE_SHADER,
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
        let Ok((device, _queue)) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("bbb-transparency-combine-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )) else {
            return None;
        };
        Some(device)
    }

    #[test]
    fn transparency_combine_samples_main_translucent_item_particle_weather_and_cloud_color_depth_layers(
    ) {
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var main_texture: texture_2d<f32>"),
            "combine shader samples the renderer-owned main color texture"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var main_depth: texture_depth_2d"),
            "combine shader samples MainDepth"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var translucent_texture: texture_2d<f32>"),
            "combine shader samples the translucent color target"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var translucent_depth: texture_depth_2d"),
            "combine shader samples TranslucentDepth"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var item_entity_texture: texture_2d<f32>"),
            "combine shader samples the item-entity color target"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var item_entity_depth: texture_depth_2d"),
            "combine shader samples ItemEntityDepth"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var particles_texture: texture_2d<f32>"),
            "combine shader samples the particles color target"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var particles_depth: texture_depth_2d"),
            "combine shader samples ParticlesDepth"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var weather_texture: texture_2d<f32>"),
            "combine shader samples the weather color target"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var weather_depth: texture_depth_2d"),
            "combine shader samples WeatherDepth"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var clouds_texture: texture_2d<f32>"),
            "combine shader samples the clouds color target"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var clouds_depth: texture_depth_2d"),
            "combine shader samples CloudsDepth"
        );
    }

    #[test]
    fn transparency_combine_uses_vanilla_depth_order_and_premultiplied_layer_blend() {
        assert!(
            TRANSPARENCY_COMBINE_SHADER
                .contains("while (jj > 0u && out.depths[jj] > out.depths[jj - 1u])"),
            "vanilla transparency sorting inserts larger depth values before nearer layers"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER
                .contains("return (dst * (1.0 - src.a)) + src.rgb"),
            "vanilla post/transparency.fsh blends premultiplied layer RGB without multiplying alpha again"
        );
    }

    #[test]
    fn transparency_combine_inserts_targets_in_vanilla_shader_order() {
        let translucent = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(translucent_texture")
            .expect("TranslucentSampler equivalent is inserted");
        let item_entity = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(item_entity_texture")
            .expect("ItemEntitySampler equivalent is inserted");
        let particles = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(particles_texture")
            .expect("ParticlesSampler equivalent is inserted");
        let weather = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(weather_texture")
            .expect("WeatherSampler equivalent is inserted");
        let clouds = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(clouds_texture")
            .expect("CloudsSampler equivalent is inserted");
        assert!(
            translucent < item_entity
                && item_entity < particles
                && particles < weather
                && weather < clouds,
            "vanilla post/transparency.fsh calls try_insert(Translucent), then ItemEntity, then Particles, then Weather, then Clouds"
        );
    }

    #[test]
    fn transparency_blit_shader_samples_internal_final_target() {
        assert!(
            TRANSPARENCY_BLIT_SHADER.contains("var final_texture: texture_2d<f32>"),
            "vanilla transparency.json writes an internal final target before blitting to main"
        );
        assert!(TRANSPARENCY_BLIT_SHADER.contains("textureSample(final_texture"));
    }

    #[test]
    fn transparency_combine_pipeline_validates_wgsl_on_wgpu_device() {
        let Some(device) = request_test_device() else {
            return;
        };
        let bind_group_layout = create_transparency_combine_bind_group_layout(&device);
        let _pipeline = create_transparency_combine_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &bind_group_layout,
        );
    }

    #[test]
    fn transparency_blit_pipeline_validates_wgsl_on_wgpu_device() {
        let Some(device) = request_test_device() else {
            return;
        };
        let bind_group_layout = create_transparency_blit_bind_group_layout(&device);
        let _pipeline = create_transparency_blit_pipeline(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            &bind_group_layout,
        );
    }
}
