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

fn try_insert(stack: ptr<function, LayerStack>, color: vec4<f32>, depth: f32) {
    if (color.a == 0.0) {
        return;
    }

    let active_count = (*stack).active_count;
    (*stack).colors[active_count] = color;
    (*stack).depths[active_count] = depth;

    var jj = active_count;
    (*stack).active_count = active_count + 1u;
    while (jj > 0u && (*stack).depths[jj] > (*stack).depths[jj - 1u]) {
        let ii = jj - 1u;
        let depth_temp = (*stack).depths[ii];
        (*stack).depths[ii] = (*stack).depths[jj];
        (*stack).depths[jj] = depth_temp;

        let color_temp = (*stack).colors[ii];
        (*stack).colors[ii] = (*stack).colors[jj];
        (*stack).colors[jj] = color_temp;

        jj = ii;
    }
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(input.position.xy);
    let main_color = textureLoad(main_texture, pixel, 0);
    let translucent_color = textureLoad(translucent_texture, pixel, 0);
    let item_entity_color = textureLoad(item_entity_texture, pixel, 0);
    let particles_color = textureLoad(particles_texture, pixel, 0);
    let weather_color = textureLoad(weather_texture, pixel, 0);
    let clouds_color = textureLoad(clouds_texture, pixel, 0);
    let optional_alpha = max(
        max(translucent_color.a, item_entity_color.a),
        max(max(particles_color.a, weather_color.a), clouds_color.a),
    );
    if (optional_alpha == 0.0) {
        return vec4<f32>(main_color.rgb, 1.0);
    }

    var stack = LayerStack(
        array<vec4<f32>, 6>(
            vec4<f32>(main_color.rgb, 1.0),
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

    try_insert(
        &stack,
        translucent_color,
        textureLoad(translucent_depth, pixel, 0),
    );
    try_insert(
        &stack,
        item_entity_color,
        textureLoad(item_entity_depth, pixel, 0),
    );
    try_insert(
        &stack,
        particles_color,
        textureLoad(particles_depth, pixel, 0),
    );
    try_insert(
        &stack,
        weather_color,
        textureLoad(weather_depth, pixel, 0),
    );
    try_insert(
        &stack,
        clouds_color,
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
    pub(super) texture: wgpu::Texture,
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    MainTarget { texture, view }
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
        // Vanilla's MainTarget is both the GUI presentation owner and a post-chain copy
        // source. The embedded transparency fallback assumes the same role when no resource
        // graph is loaded, so camera/screen effects must be able to snapshot it before their
        // in-place result pass.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
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
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
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
    use std::sync::mpsc;

    use super::{
        create_transparency_blit_bind_group_layout, create_transparency_blit_pipeline,
        create_transparency_combine_bind_group_layout, create_transparency_combine_pipeline,
        TRANSPARENCY_BLIT_SHADER, TRANSPARENCY_COMBINE_SHADER,
    };

    fn request_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bbb-transparency-combine-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            ..Default::default()
        }))
        .ok()
    }

    fn create_test_color_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        color: [u8; 4],
    ) -> wgpu::Texture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &color,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        texture
    }

    fn create_test_depth_texture(device: &wgpu::Device, label: &str) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn expected_transparency_pixel(colors: [[u8; 4]; 6], depths: [f32; 6]) -> [u8; 4] {
        let normalized = colors.map(|color| color.map(|channel| f32::from(channel) / 255.0));
        let mut layers = vec![(
            [normalized[0][0], normalized[0][1], normalized[0][2], 1.0],
            depths[0],
        )];
        layers.extend(
            normalized[1..]
                .iter()
                .zip(&depths[1..])
                .filter(|(color, _)| color[3] != 0.0)
                .map(|(color, depth)| (*color, *depth)),
        );
        layers.sort_by(|left, right| right.1.total_cmp(&left.1));

        let mut accumulated: [f32; 3] = layers[0].0[..3].try_into().unwrap();
        for (color, _) in &layers[1..] {
            for channel in 0..3 {
                accumulated[channel] = accumulated[channel] * (1.0 - color[3]) + color[channel];
            }
        }
        [
            (accumulated[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (accumulated[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (accumulated[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            255,
        ]
    }

    fn render_transparency_pixel(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        colors: [[u8; 4]; 6],
        depths: [f32; 6],
    ) -> [u8; 4] {
        const PADDED_BYTES_PER_ROW: u32 = 256;
        let color_textures = colors
            .into_iter()
            .enumerate()
            .map(|(index, color)| {
                create_test_color_texture(
                    device,
                    queue,
                    &format!("bbb-transparency-test-color-{index}"),
                    color,
                )
            })
            .collect::<Vec<_>>();
        let color_views = color_textures
            .iter()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect::<Vec<_>>();
        let depth_textures = depths
            .iter()
            .enumerate()
            .map(|(index, _)| {
                create_test_depth_texture(device, &format!("bbb-transparency-test-depth-{index}"))
            })
            .collect::<Vec<_>>();
        let depth_views = depth_textures
            .iter()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect::<Vec<_>>();
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-transparency-test-output"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-transparency-test-readback"),
            size: u64::from(PADDED_BYTES_PER_ROW),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let layout = create_transparency_combine_bind_group_layout(device);
        let pipeline =
            create_transparency_combine_pipeline(device, wgpu::TextureFormat::Rgba8Unorm, &layout);
        let bind_group_entries = color_views
            .iter()
            .zip(&depth_views)
            .enumerate()
            .flat_map(|(index, (color, depth))| {
                let color_binding = u32::try_from(index * 2).unwrap();
                [
                    wgpu::BindGroupEntry {
                        binding: color_binding,
                        resource: wgpu::BindingResource::TextureView(color),
                    },
                    wgpu::BindGroupEntry {
                        binding: color_binding + 1,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                ]
            })
            .collect::<Vec<_>>();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bbb-transparency-test-bind-group"),
            layout: &layout,
            entries: &bind_group_entries,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bbb-transparency-test-encoder"),
        });
        for (view, depth) in depth_views.iter().zip(depths) {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-transparency-test-depth-clear"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-transparency-test-combine"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(PADDED_BYTES_PER_ROW),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        receiver.recv().unwrap().unwrap();
        let mapped = slice.get_mapped_range().expect("mapped transparency pixel");
        let pixel = mapped[..4].try_into().unwrap();
        drop(mapped);
        readback.unmap();
        pixel
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
        assert!(TRANSPARENCY_COMBINE_SHADER
            .contains("while (jj > 0u && (*stack).depths[jj] > (*stack).depths[jj - 1u])"));
        assert!(TRANSPARENCY_COMBINE_SHADER.contains("while (ii < stack.active_count)"));
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("ptr<function, LayerStack>"),
            "Vanilla mutates its layer arrays in place instead of copying the full stack per insert"
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
            .find("&stack,\n        translucent_color")
            .expect("TranslucentSampler equivalent is inserted");
        let item_entity = TRANSPARENCY_COMBINE_SHADER
            .find("&stack,\n        item_entity_color")
            .expect("ItemEntitySampler equivalent is inserted");
        let particles = TRANSPARENCY_COMBINE_SHADER
            .find("&stack,\n        particles_color")
            .expect("ParticlesSampler equivalent is inserted");
        let weather = TRANSPARENCY_COMBINE_SHADER
            .find("&stack,\n        weather_color")
            .expect("WeatherSampler equivalent is inserted");
        let clouds = TRANSPARENCY_COMBINE_SHADER
            .find("&stack,\n        clouds_color")
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
    fn transparency_combine_empty_fast_path_precedes_depth_fetches_without_changing_active_path() {
        let fast_return = TRANSPARENCY_COMBINE_SHADER
            .find("if (optional_alpha == 0.0)")
            .expect("five-layer empty fast path exists");
        let main_depth = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(main_depth, pixel, 0)")
            .expect("active path still fetches main depth");
        assert!(
            fast_return < main_depth,
            "the all-empty path returns before any depth texture fetch"
        );
        for depth in [
            "translucent_depth",
            "item_entity_depth",
            "particles_depth",
            "weather_depth",
            "clouds_depth",
        ] {
            assert!(
                TRANSPARENCY_COMBINE_SHADER[fast_return..]
                    .contains(&format!("textureLoad({depth}, pixel, 0)")),
                "active path retains the unconditional {depth} fetch"
            );
        }
    }

    #[test]
    fn transparency_combine_empty_and_active_pixels_match_original_blend_contract() {
        let Some((device, queue)) = request_test_device() else {
            return;
        };
        let empty_colors = [
            [31, 67, 101, 19],
            [201, 31, 17, 0],
            [7, 199, 53, 0],
            [91, 13, 211, 0],
            [44, 88, 132, 0],
            [255, 127, 63, 0],
        ];
        let empty_depths = [0.37, 0.92, 0.11, 0.73, 0.44, 0.28];
        let empty_actual = render_transparency_pixel(&device, &queue, empty_colors, empty_depths);
        assert_eq!(
            empty_actual,
            expected_transparency_pixel(empty_colors, empty_depths),
            "all optional layers empty returns the opaque main pixel"
        );

        let active_colors = [
            [40, 72, 104, 255],
            [64, 16, 8, 128],
            [0, 0, 0, 0],
            [12, 48, 24, 64],
            [0, 0, 0, 0],
            [24, 12, 60, 96],
        ];
        let active_depths = [0.42, 0.81, 0.17, 0.23, 0.65, 0.58];
        let active_actual =
            render_transparency_pixel(&device, &queue, active_colors, active_depths);
        let active_expected = expected_transparency_pixel(active_colors, active_depths);
        for channel in 0..4 {
            assert!(
                active_actual[channel].abs_diff(active_expected[channel]) <= 1,
                "active-layer channel {channel} differs: actual={active_actual:?}, expected={active_expected:?}"
            );
        }
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
        let Some((device, _queue)) = request_test_device() else {
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
        let Some((device, _queue)) = request_test_device() else {
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
