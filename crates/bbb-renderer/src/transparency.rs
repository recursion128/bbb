use crate::{clouds::CloudTarget, gpu::DepthTarget};

pub(super) const TRANSPARENCY_COMBINE_SHADER: &str = r#"
@group(0) @binding(0)
var main_texture: texture_2d<f32>;

@group(0) @binding(1)
var main_depth: texture_depth_2d;

@group(0) @binding(2)
var clouds_texture: texture_2d<f32>;

@group(0) @binding(3)
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

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(input.position.xy);
    let main_color = vec4<f32>(textureLoad(main_texture, pixel, 0).rgb, 1.0);
    let main_depth_value = textureLoad(main_depth, pixel, 0);
    let clouds_color = textureLoad(clouds_texture, pixel, 0);
    let clouds_depth_value = textureLoad(clouds_depth, pixel, 0);

    if (clouds_color.a == 0.0) {
        return main_color;
    }

    if (clouds_depth_value > main_depth_value) {
        return vec4<f32>(blend(clouds_color.rgb, main_color), 1.0);
    }

    return vec4<f32>(blend(main_color.rgb, clouds_color), 1.0);
}
"#;

pub(super) struct MainTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
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

pub(super) fn create_transparency_combine_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    main_target: &MainTarget,
    main_depth: &DepthTarget,
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
                resource: wgpu::BindingResource::TextureView(&cloud_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
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
    fn transparency_combine_samples_main_and_cloud_color_depth_layers() {
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var main_texture: texture_2d<f32>"),
            "combine shader samples the renderer-owned main color texture"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER.contains("var main_depth: texture_depth_2d"),
            "combine shader samples MainDepth"
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
            TRANSPARENCY_COMBINE_SHADER.contains("clouds_depth_value > main_depth_value"),
            "vanilla transparency sorting inserts larger depth values before nearer layers"
        );
        assert!(
            TRANSPARENCY_COMBINE_SHADER
                .contains("return (dst * (1.0 - src.a)) + src.rgb"),
            "vanilla post/transparency.fsh blends premultiplied layer RGB without multiplying alpha again"
        );
    }
}
