use crate::{
    clouds::CloudTarget,
    gpu::{DepthTarget, DEPTH_FORMAT, DEPTH_TARGET_USAGE},
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
var clouds_texture: texture_2d<f32>;

@group(0) @binding(5)
var clouds_depth: texture_depth_2d;

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

fn blend(dst: vec3<f32>, src: vec4<f32>) -> vec3<f32> {
    return (dst * (1.0 - src.a)) + src.rgb;
}

struct LayerStack {
    colors: array<vec4<f32>, 6>,
    depths: array<f32, 6>,
    active: u32,
};

fn try_insert(stack: LayerStack, color: vec4<f32>, depth: f32) -> LayerStack {
    var out = stack;
    if (color.a == 0.0) {
        return out;
    }

    out.colors[out.active] = color;
    out.depths[out.active] = depth;

    var jj = out.active;
    out.active = out.active + 1u;
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
        textureLoad(clouds_texture, pixel, 0),
        textureLoad(clouds_depth, pixel, 0),
    );

    var texel_accum = stack.colors[0].rgb;
    var ii = 1u;
    while (ii < stack.active) {
        texel_accum = blend(texel_accum, stack.colors[ii]);
        ii = ii + 1u;
    }

    return vec4<f32>(texel_accum.rgb, 1.0);
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

pub(super) struct TransparencyCombineBindGroup {
    pub(super) bind_group: wgpu::BindGroup,
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

pub(super) fn create_transparency_combine_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    main_target: &MainTarget,
    main_depth: &DepthTarget,
    translucent_target: &TranslucentTarget,
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
                resource: wgpu::BindingResource::TextureView(&cloud_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 5,
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
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-transparency-combine-shader"),
        source: wgpu::ShaderSource::Wgsl(TRANSPARENCY_COMBINE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-transparency-combine-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-transparency-combine-pipeline"),
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
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

#[cfg(test)]
mod tests {
    use super::TRANSPARENCY_COMBINE_SHADER;

    #[test]
    fn transparency_combine_samples_main_translucent_and_cloud_color_depth_layers() {
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
    fn transparency_combine_inserts_translucent_before_clouds_like_vanilla_shader() {
        let translucent = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(translucent_texture")
            .expect("TranslucentSampler equivalent is inserted");
        let clouds = TRANSPARENCY_COMBINE_SHADER
            .find("textureLoad(clouds_texture")
            .expect("CloudsSampler equivalent is inserted");
        assert!(
            translucent < clouds,
            "vanilla post/transparency.fsh calls try_insert(Translucent) before try_insert(Clouds)"
        );
    }
}
