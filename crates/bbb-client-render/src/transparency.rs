use crate::texture_sampling::create_nearest_clamp_sampler;
use crate::{
    clouds::CloudTarget,
    gpu::{DepthTarget, DEPTH_FORMAT, DEPTH_TARGET_USAGE},
    pipeline_builder::RenderPipelineBuilder,
};

pub(super) const TRANSPARENCY_COMBINE_SHADER: &str = concat!(
    crate::wgsl::fullscreen_triangle_position!(),
    r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) omitted: u32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) omitted: u32) -> VertexOut {
    return VertexOut(vec4<f32>(fullscreen_position(vertex_index), 0.0, 1.0), omitted);
}

@group(0) @binding(0)
var main_texture: texture_2d<f32>;

@group(0) @binding(1)
var main_depth: texture_2d<f32>;

@group(0) @binding(2)
var translucent_texture: texture_2d<f32>;

@group(0) @binding(3)
var translucent_depth: texture_2d<f32>;

@group(0) @binding(4)
var item_entity_texture: texture_2d<f32>;

@group(0) @binding(5)
var item_entity_depth: texture_2d<f32>;

@group(0) @binding(6)
var particles_texture: texture_2d<f32>;

@group(0) @binding(7)
var particles_depth: texture_2d<f32>;

@group(0) @binding(8)
var weather_texture: texture_2d<f32>;

@group(0) @binding(9)
var weather_depth: texture_2d<f32>;

@group(0) @binding(10)
var clouds_texture: texture_2d<f32>;

@group(0) @binding(11)
var clouds_depth: texture_2d<f32>;

fn blend(dst: vec3<f32>, src: vec4<f32>) -> vec3<f32> {
    return (dst * (1.0 - src.a)) + src.rgb;
}

struct LayerStack {
    colors: array<vec4<f32>, 4>,
    depths: array<f32, 4>,
    active_count: u32,
    opaque_depth: f32,
};

fn try_insert(stack: ptr<function, LayerStack>, color: vec4<f32>, depth: f32) {
    // Main has alpha one. Equal-depth features follow it in the stable draw order.
    if (color.a == 0.0 || depth > (*stack).opaque_depth) {
        return;
    }

    let active_count = (*stack).active_count;
    // Fixed bounds let FXC unroll local-array lvalues.
    for (var index = 0u; index < 4u; index = index + 1u) {
        if (index == active_count) {
            (*stack).colors[index] = color;
            (*stack).depths[index] = depth;
        }
    }

    (*stack).active_count = active_count + 1u;
    for (var jj = 3u; jj > 0u; jj = jj - 1u) {
        if (jj <= active_count && (*stack).depths[jj] > (*stack).depths[jj - 1u]) {
            let ii = jj - 1u;
            let depth_temp = (*stack).depths[ii];
            (*stack).depths[ii] = (*stack).depths[jj];
            (*stack).depths[jj] = depth_temp;

            let color_temp = (*stack).colors[ii];
            (*stack).colors[ii] = (*stack).colors[jj];
            (*stack).colors[jj] = color_temp;
        }
    }
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(input.position.xy);
    var stack: LayerStack;
    stack.opaque_depth = textureLoad(main_depth, pixel, 0).r;

    try_insert(
        &stack,
        textureLoad(translucent_texture, pixel, 0),
        textureLoad(translucent_depth, pixel, 0).r,
    );
    if ((input.omitted & 1u) == 0u) {
        try_insert(
            &stack,
            textureLoad(item_entity_texture, pixel, 0),
            textureLoad(item_entity_depth, pixel, 0).r,
        );
    }
    if ((input.omitted & 2u) == 0u) {
        try_insert(
            &stack,
            textureLoad(particles_texture, pixel, 0),
            textureLoad(particles_depth, pixel, 0).r,
        );
    }
    if ((input.omitted & 4u) == 0u) {
        try_insert(
            &stack,
            textureLoad(weather_texture, pixel, 0),
            textureLoad(weather_depth, pixel, 0).r,
        );
    }
    var cloud = vec4<f32>(0.0);
    var cloud_depth = 0.0;
    var cloud_pending = false;
    if ((input.omitted & 8u) == 0u) {
        cloud = textureLoad(clouds_texture, pixel, 0);
        cloud_depth = textureLoad(clouds_depth, pixel, 0).r;
        cloud_pending = cloud.a != 0.0 && cloud_depth <= stack.opaque_depth;
    }

    // Clouds are last in Vanilla's insertion order. Merge them during blending;
    // equal-depth layers from the stack must be blended first.
    var texel_accum = textureLoad(main_texture, pixel, 0).rgb;
    for (var ii = 0u; ii < 4u; ii = ii + 1u) {
        if (ii < stack.active_count) {
            if (cloud_pending && cloud_depth > stack.depths[ii]) {
                texel_accum = blend(texel_accum, cloud);
                cloud_pending = false;
            }
            texel_accum = blend(texel_accum, stack.colors[ii]);
        }
    }
    if (cloud_pending) {
        texel_accum = blend(texel_accum, cloud);
    }
    return vec4<f32>(texel_accum.rgb, 1.0);
}
"#,
);

pub(super) const TRANSPARENCY_BLIT_SHADER: &str = concat!(
    crate::wgsl::fullscreen_triangle!(),
    r#"
@group(0) @binding(0)
var final_texture: texture_2d<f32>;

@group(0) @binding(1)
var final_sampler: sampler;

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(final_texture, final_sampler, input.uv);
}
"#,
);

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

// Raw depth uses unfilterable float bindings so GLSL can emit exact texel loads.
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
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
    let sampler = create_nearest_clamp_sampler(device, "bbb-transparency-final-sampler");
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

#[allow(clippy::too_many_arguments)]
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
    hand_depth: bool,
) -> wgpu::RenderPipeline {
    create_transparency_fullscreen_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-transparency-combine-shader",
        "bbb-transparency-combine-pipeline-layout",
        "bbb-transparency-combine-pipeline",
        TRANSPARENCY_COMBINE_SHADER,
        hand_depth,
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
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_transparency_fullscreen_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader_label: &str,
    pipeline_layout_label: &str,
    pipeline_label: &str,
    shader_source: &str,
    hand_depth: bool,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, pipeline_label)
        .shader(shader_label, shader_source)
        .layout(pipeline_layout_label, &[bind_group_layout])
        .depth_stencil(hand_depth.then(|| {
            crate::pipeline_builder::depth_stencil_state(
                DEPTH_FORMAT,
                false,
                wgpu::CompareFunction::Always,
            )
        }))
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
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .ok()?;
        let Ok((device, _queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bbb-transparency-combine-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            }))
        else {
            return None;
        };
        Some(device)
    }

    #[test]
    fn transparency_combine_preserves_sorted_stable_premultiplied_pixels() {
        let Some(renderer) = crate::Renderer::new_offscreen(1, 1) else {
            return;
        };
        let device = &renderer.device;
        let texture = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bbb-transparency-pixel-test"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let colors: Vec<_> = (0..6)
            .map(|_| {
                texture(
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                )
            })
            .collect();
        let depths: Vec<_> = (0..6)
            .map(|_| {
                texture(
                    crate::gpu::DEPTH_FORMAT,
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                )
            })
            .collect();
        let color_views: Vec<_> = colors
            .iter()
            .map(|t| t.create_view(&Default::default()))
            .collect();
        let depth_views: Vec<_> = depths
            .iter()
            .map(|t| t.create_view(&Default::default()))
            .collect();
        let output = texture(
            renderer.config.format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let output_view = output.create_view(&Default::default());
        let layout = create_transparency_combine_bind_group_layout(device);
        let entries: Vec<_> = (0..6)
            .flat_map(|i| {
                [
                    wgpu::BindGroupEntry {
                        binding: i as u32 * 2,
                        resource: wgpu::BindingResource::TextureView(&color_views[i]),
                    },
                    wgpu::BindGroupEntry {
                        binding: i as u32 * 2 + 1,
                        resource: wgpu::BindingResource::TextureView(&depth_views[i]),
                    },
                ]
            })
            .collect();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bbb-transparency-pixel-test"),
            layout: &layout,
            entries: &entries,
        });
        let pipeline =
            create_transparency_combine_pipeline(device, renderer.config.format, &layout, false);
        let layer_colors = [
            [0.2, 0.4, 0.6, 1.0],
            [0.4, 0.0, 0.0, 0.5],
            [0.0, 0.4, 0.0, 0.5],
            [0.0, 0.0, 0.4, 0.5],
            [0.2, 0.2, 0.0, 0.5],
            [0.2, 0.0, 0.2, 0.5],
        ];
        for (name, layer_depths, transparent, omitted, expected) in [
            (
                "mixed depths",
                [0.7, 0.9, 0.3, 0.8, 0.5, 0.1],
                false,
                0,
                [70u8, 77, 70, 255],
            ),
            (
                "cloud blends between terrain and the front particle",
                [0.9, 0.8, 0.0, 0.2, 0.0, 0.5],
                false,
                5,
                [57, 13, 146, 255],
            ),
            (
                "equal depths including opaque main",
                [0.5; 6],
                false,
                0,
                [84, 41, 81, 255],
            ),
            (
                "omitted clouds retain a previous foreground texel",
                [0.5, 0.9, 0.8, 0.7, 0.6, 0.1],
                false,
                8,
                [51, 102, 153, 255],
            ),
            (
                "zero alpha",
                [0.7, 0.9, 0.3, 0.8, 0.5, 0.1],
                true,
                0,
                [51, 102, 153, 255],
            ),
            (
                "five visible layers in reverse depth order",
                [0.9, 0.1, 0.2, 0.3, 0.4, 0.5],
                false,
                0,
                [113, 60, 33, 255],
            ),
            (
                "opaque main covers all five layers",
                [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
                false,
                0,
                [51, 102, 153, 255],
            ),
            (
                "omitted item, particle and weather layers contain stale colors",
                [0.7, 0.5, 0.5, 0.5, 0.5, 0.5],
                false,
                7,
                [115, 26, 89, 255],
            ),
        ] {
            let mut encoder = device.create_command_encoder(&Default::default());
            for i in 0..6 {
                let [r, g, b, a] = layer_colors[i];
                let a = if transparent && i > 0 { 0.0 } else { a };
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-transparency-pixel-test-clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_views[i],
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_views[i],
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(layer_depths[i]),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
            }
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-transparency-pixel-test-combine"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &output_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..3, omitted..omitted + 1);
            }
            let pending = renderer
                .prepare_screenshot_copy(&mut encoder, &output)
                .unwrap();
            renderer.queue.submit(Some(encoder.finish()));
            let actual = renderer
                .read_screenshot_pixels(pending)
                .unwrap()
                .pixel(0, 0);
            eprintln!("transparency {name}: {actual:?}");
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "{name}: expected {expected:?}, got {actual:?}"
            );
        }
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
            false,
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
